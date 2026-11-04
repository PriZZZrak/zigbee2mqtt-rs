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

## Checksum, index metadata and schedule/retry policy

- Added a dependency-free SHA-512 implementation (verified against the standard
  `abc` vector), `validate_firmware` (checksum over the full file, then strips any
  vendor prefix before the OTA magic) and `OtaManager::start_checked`.
- `OtaIndexEntry`/`parse_index` deserialize the published index fields;
  `select_image` mirrors herdsman `findMatchingOtaImage` filters: manufacturer,
  image type, min/max file version, hardware min/max, modelId and
  manufacturerName list (first matching entry in index order).
- `OtaCheckScheduler` implements the per-device check interval
  (`ota.update_check_interval` in the app); `OtaRetryPolicy` provides bounded
  attempts with backoff.
- Bridge check/update accept an optional `sha512` field and validate it before
  parsing/starting; a wrong checksum returns an `error` response.
- `OtaIndexProvider` loads HTTP(S) or local indexes through an `IndexFetcher`
  (`CommandFetcher` uses curl/wget/uclient-fetch with an 8 MiB cap), puts override
  entries first, and tolerates main-index failure only when an override exists.
- `OtaManager::check_for_update` wires the check scheduler: the first query is
  checked, subsequent checks are throttled by the interval, and the selected
  index entry is returned. `set_check_interval` configures it.
- Local index files are wired into the bridge: `index` + `image_type` (and
  optional manufacturer/file/hardware fields) select an entry, resolve its local
  URL relative to the config directory, verify `sha512`, and use the selected
  image for check/update. Responses include `source` and `release_notes`.

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

OTA tests now also cover SHA-512, checksum/prefix validation, index
parsing/selection filters, check scheduling, retry backoff, HTTP-index override
precedence and main-index fallback. Twelve core
OTA/manager tests plus three bridge-request tests:

- `ota_bridge_check_and_update_requests_end_to_end`: real bridge request
  payloads → check info, update state `{state: updating, progress: 0}`, abort →
  idle.
- `ota_bridge_request_error_paths`: unknown device, invalid hex, unsupported
  schedule request.
- `ota_progress_payload_matches_state_contract`: 0.5 → 50 while updating and
  completion → idle/100.

`cargo test --locked` → **370 passed** (338 library, 2 binary, 30 compatibility);
clippy exit 0 with five existing warnings; release build OK.

## Divergences and gaps (not verified)

- The bridge request handler is covered directly with a fake coordinator/manager,
  but not through the MQTT event loop or a broker; state publication is asserted
  as payload fields, not on the wire.
- `check` reports any valid image as available; it does not compare device
  identity or query the device first.
- HTTP(S) index loading is implemented through a system downloader, but the
  provider/scheduler are only exercised at manager level and are not wired into
  the bridge's automatic device-query path yet. Local bridge index handling still
  requires explicit manufacturer/image metadata until the query is used.
- Override/main selection and throttling are tested with a fake fetcher; no real
  HTTPS fetch, ETag/cache, persistence or downgrade selection.
- The retry policy is unit-tested but not wired to automatic checks.
- The SHA-512 implementation is new code and needs independent review/fuzzing
  before security-sensitive use.
- No image file checksum/signature validation, manufacturer/hardware applicability
  beyond identity, block-size negotiation, retry/timeout policy, upgrade delay or
  progress publication.
- `minimumBlockPeriod` is parsed but never enforced; ImagePageRequest 0x04 and
  query-specific optional fields are not implemented.
- No Node differential test or hardware OTA (explicitly requires user approval);
  OTA-001 stays `IN_PROGRESS`.
