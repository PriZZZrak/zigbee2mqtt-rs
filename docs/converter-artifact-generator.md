# Converter artifact generator and definition resolver

Status: phases 1–2 implemented and measured; lazy loader/cache and production
transport are the next milestone.
Implementation:
`tools/artifact-generator/`, `crates/converter-artifact/`,
`crates/zigbee-apiarist/tests/differential_zcl_pinned.rs`.
Research baseline: [`converter-artifact-research.md`](converter-artifact-research.md).

No production bridge wiring was changed, no network fetch was added for the
runtime, and no coordinator/MQTT/HA operation was executed.

## 1. Generator

`tools/artifact-generator/generate.js` is deterministic and driven by the pinned
dist tree plus explicit revision/ABI arguments. It:

- loads all 4619 definitions and the pinned `models-index.json`;
- emits a fingerprint-aware `index.json` and `index.z2idx1`;
- emits a rewritten `dist/models-index.json` whose entries point at artifact
  fragments/local indices when a module is split;
- emits `graph.json` with per-module bytes, sha256 and direct require edges;
- emits `manifest.json` with sha256/bytes for index, graph, shared tree and every
  module, plus upstream revisions, host ABI, engine and ZCL catalog hash;
- copies only `dist/index.js`, `dist/converters`, `dist/lib` into the package
  shared tree; device modules remain artifact entries, not opkg content;
- splits `tuya.js` with a real Babel AST parser and gates the result on
  prepared-definition equality and, when a runtime probe is supplied,
  behavioral differential. Any failure falls back to the original module and
  records the reason in the manifest.

CLI and reproduction commands are in
[`../tools/artifact-generator/README.md`](../tools/artifact-generator/README.md).

### Determinism

Two independent runs produced identical `manifest.json` and
`index.z2idx1` hashes:

```text
manifest  640a708d...d37c356
index     49f5186f...c2c182
```

## 2. Resolver

`crates/converter-artifact` parses both the compact JSON projection and
`z2idx1`, with full bounds checking, and implements the pinned upstream
resolution semantics:

- key lookup: lowercase model key, `null` for missing/empty modelID, then
  `normalizeModelID` (strip from first NUL, trim);
- single candidate with `zigbeeModel` shortcut;
- fingerprint matching for all fields: applicationVersion, manufacturerID,
  type, dateCode, hardwareVersion, manufacturerName, modelID, powerSource,
  softwareBuildID, stackVersion, zclVersion, IEEE regex, endpoint
  IDs/profile/device/clusters;
- JS truthiness semantics for empty string fields (empty is ignored);
- highest fingerprint priority wins; equal-priority tie keeps the first
  candidate/fingerprint;
- fallback to exact/normalized `zigbeeModel`;
- white-label overlay (`model`, `vendor`, `description`) after resolution;
- coordinator returns no definition.

### Differential result

Oracle: `tools/artifact-generator/resolver-oracle.js` calls pinned
`findByDevice` for **11,562 profiles**:

- all 5,069 index keys;
- every definition fingerprint;
- every white-label fingerprint;
- manufacturer/model/type mutation cases;
- additional fields (manufacturerID, powerSource, versions, dateCode,
  softwareBuildID, endpoints) and the null-model key.

| Format | Bytes | Parse (Routerich) | RSS | Failures |
|---|---:|---:|---:|---:|
| compact JSON | 1,454,123 | 86–124 ms | 7.8 MB | 0 |
| `z2idx1` | 927,458 | 68–86 ms | 5.4–6.1 MB | 0 |

`crates/converter-artifact/src/bin/resolver_diff.rs` compares both formats
against the same oracle; both report **0/11,562** failures. `z2idx1` is chosen:
34% smaller, lower memory, equal or faster parse on the target.

The earlier `models-index` path for the real `TS0601/_TZE200_locansqn` device
loaded 73 files/4.16 MB/8.17 s. The resolver returns the pinned result without
loading a device module; with white-label overlay it returns `TH01Z` (Tuya),
and the underlying definition is `nous.js`.

## 3. tuya split

- 571 definitions, 8 fragments of 150–182 KB plus a 107 KB common module.
- Original definition text and order preserved; fragment local indices are
  written into the index/manifest.
- Prepared-definition equality: **571/571, 0 differences**.
- Behavioral differential through the real converter runtime: fromZigbee, set,
  configure and onEvent, including definitions from multiple fragments:
  **4/4 scenarios, 0 functional differences** (timing fields ignored).
- If either gate fails, the generator removes the fragments and records
  `status: "fallback"` for that module. Other large files are not split by
  default; `--split-files` is opt-in.

## 4. Manifest and integrity

`manifest.json` records:

- upstream z2m/commit/herdsman/converters/Node revisions;
- host ABI, engine/version, required features, ZCL catalog sha256;
- index/JSON/graph bytes and sha256, key/definition/candidate counts;
- shared tree file list with sha256/bytes and a canonical tree hash;
- every module with bytes/sha256/definition count;
- split verification/behavior results and fallback reasons;
- cache/offline policy and compatibility range.

`converter-artifact::Manifest` verifies schema, ABI/engine compatibility, index,
graph, shared files, shared tree hash and individual module hashes. The
manifest's own `signature` field is explicitly `hashOnly: true`; it is an
integrity checksum, **not** a cryptographic signature. Network module downloads
must be checked against this trusted manifest before caching/evaluation.

## 5. ZCL/ZDO differential

`tools/oracle/generate-zcl-pinned.mjs` generates from pinned
zigbee-herdsman 11.2.0:

- **413 ZCL cases** (206 encode, 207 decode) across all 129 clusters,
  commands/commandsResponse, manufacturer-specific cluster data, global
  commands and two corrupted/truncated buffers;
- **60 ZDO constants** with names/IDs.

`differential_zcl_pinned.rs` passes every valid corpus case. Two generated
greenPower/telecom cases produce Rust `INVALID_FIELD` while upstream encodes a
payload; they are explicitly excluded from the zero-diff gate and recorded with
case ids in
`tools/bench/results/artifact-generator/zcl-differential.json`. They are the
only unexplained-at-codec-boundary items and are a Phase-5 fix, not hidden.
Unknown inbound clusters follow the existing raw-diagnostic path; unknown
outbound clusters remain a documented next-stage error path (no guessed
encoding).

## 6. Measured artifact

| Component | Bytes |
|---|---:|
| `z2m-rs` aarch64 musl | 18,313,424 raw / 6,069,341 gzip |
| shared JS tree | 2,443,459 |
| `index.z2idx1` | 927,458 |
| manifest + graph | ~110,000 |
| staged data tar.gz | **6,900,732** |
| paired cache (nous + external + one tuya fragment set) | 337,043 raw / ~61 KB gzip |

The current Node opkg is 12.24 MB compressed / 53.6 MB installed; the overlay
still has only ~1.1 MB free. The Rust artifact is smaller but cannot be
installed side-by-side without the agreed Node removal/external-storage step.

## 7. Limits

- Full oracle generation is a build/research step; the 11,562-case oracle is
  not committed. The small automatic fixtures plus the generator/`resolver_diff`
  commands reproduce the full gate.
- Two ZCL payload-validation anomalies remain documented; they need a codec
  decision before enabling greenPower/telecom commands.
- ZDO frame corpus currently covers constants plus existing wire/zdo suites; a
  full ZDO request/response payload oracle remains next-stage work.
- No lazy loader/cache is active in the bridge; the artifact is not wired to
  production startup yet.
- No network fetch, cache manager, OpenWrt package migration or HIL.

## 8. GO / NO-GO

**GO** for the lazy loader and cache manager stage:

1. `z2idx1` resolver is 0-diff on the full oracle;
2. the generator is deterministic and hash-verified;
3. split gates and fallback are implemented;
4. manifest/ABI/hash verification is implemented;
5. ZCL/ZDO differential is green except two documented codec anomalies.

**NO-GO for production enablement** until:

1. the two ZCL anomalies are fixed or explicitly excluded from supported
   commands;
2. the lazy loader pins paired modules and verifies hashes against the manifest;
3. cache rollback/last-known-good and offline-paired behavior are tested;
4. the ZDO frame corpus is expanded;
5. the overlay migration is agreed and HIL is separately approved.
