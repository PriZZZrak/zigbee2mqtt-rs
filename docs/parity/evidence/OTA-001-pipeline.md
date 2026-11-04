# OTA-001 — ZCL OTA transfer pipeline

Rust implementation: `src/zigbee/zcl/clusters/ota.rs`. Behavior: OTA-001;
plan FEAT-02.

## Source contract

Pinned app/herdsman:

- ZCL OTA Upgrade cluster 0x0019 commands: QueryNextImageRequest 0x01/Response
  0x02, ImageBlockRequest 0x03/Response 0x05, UpgradeEndRequest 0x06/Response
  0x07.
- App `lib/extension/otaUpdate.ts` selects/checks image metadata and calls
  herdsman device OTA; herdsman model/device OTA drives the block transfer,
  timeouts, progress and upgrade-end handling.
- `test/device-ota.test.ts` and related fixtures are the reference for transfer
  sequences; they have not been executed in this environment.

## Bridge/metadata integration

- `src/ota.rs` parses the 56-byte OTA file header and owns per-device
  `OtaTransfer`s.
- Bridge `device/ota_update/check` accepts a hex image payload, validates the
  header and responds with manufacturer/image type/version/size.
- `device/ota_update/update` starts a transfer, publishes
  `update: {state: "updating", progress: 0}`, and `update/abort`/`unschedule`
  aborts it.
- Incoming ZCL cluster 0x0019 frames are routed to the manager: QueryNextImage,
  ImageBlock and UpgradeEnd responses are sent back to the device, progress is
  published (rounded percent), and completion returns the update state to idle.
- The image source is currently an explicit hex payload; network URL/index
  metadata and persistent firmware storage are not implemented.

## Rust change (transport-independent core)

- Typed parsers for QueryNextImageRequest, ImageBlockRequest (including optional
  node address/minimum block period fields) and UpgradeEndRequest.
- Builders for QueryNextImageResponse (success/no-image), ImageBlockResponse and
  UpgradeEndResponse with the cluster-specific server→client frame.
- `OtaTransfer` state machine:
  - validates manufacturer/image type/file version;
  - serves requested byte ranges up to `max_data_size`, supports retries at an
    earlier offset;
  - tracks offset/progress/completion;
  - rejects zero-length blocks and offsets beyond the image;
  - requires a fully transferred image with matching identity before accepting
    UpgradeEndRequest.
- Fake-coordinator replay fixture: query request bytes → query response;
  repeated block requests → sequential chunk sizes 4/4/2; upgrade end → response.

## Tests and evidence

Twelve OTA tests: nine core/manager tests (header parsing, query/blocks/end replay)
plus three bridge-request tests:

- `ota_bridge_check_and_update_requests_end_to_end`: real bridge request
  payloads → check info, update state `{state: updating, progress: 0}`, abort →
  idle.
- `ota_bridge_request_error_paths`: unknown device, invalid hex, unsupported
  schedule request.
- `ota_progress_payload_matches_state_contract`: 0.5 → 50 while updating and
  completion → idle/100.

`cargo test --locked` → **358 passed** (326 library, 2 binary, 30 compatibility);
clippy exit 0 with five existing warnings; release build OK.

## Divergences and gaps (not verified)

- The bridge request handler is covered directly with a fake coordinator/manager,
  but not through the MQTT event loop or a broker; state publication is asserted
  as payload fields, not on the wire.
- `check` reports any valid image as available; it does not compare device
  identity or query the device first.
- Only hex image payloads are supported; URL/index metadata, persistence,
  downgrade rules and scheduling are missing.
- No image file checksum/signature validation, manufacturer/hardware applicability
  beyond identity, block-size negotiation, retry/timeout policy, upgrade delay or
  progress publication.
- `minimumBlockPeriod` is parsed but never enforced; ImagePageRequest 0x04 and
  query-specific optional fields are not implemented.
- No Node differential test or hardware OTA (explicitly requires user approval);
  OTA-001 stays `IN_PROGRESS`.
