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

Seven tests: query parsing/response, block progress/fragmentation, wrong identity,
premature upgrade end, block/upgrade-end parsing, and full replay.

`cargo test --locked` → **353 passed** (321 library, 2 binary, 30 compatibility);
clippy exit 0 with five existing warnings; release build OK.

## Divergences and gaps (not verified)

- No bridge/MQTT integration: `device/ota_update/*` still returns placeholder
  responses and no image source/metadata/cache is implemented.
- No image file checksum/signature validation, manufacturer/hardware applicability
  beyond identity, block-size negotiation, retry/timeout policy, upgrade delay or
  progress publication.
- `minimumBlockPeriod` is parsed but never enforced; ImagePageRequest 0x04 and
  query-specific optional fields are not implemented.
- No Node differential test or hardware OTA (explicitly requires user approval);
  OTA-001 stays `IN_PROGRESS`.
