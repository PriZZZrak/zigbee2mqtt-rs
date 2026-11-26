# Converter artifact generator and definition resolver

Status: phases 1–2 (generator, resolver) and the lazy loader/cache, cache
consistency and HTTPS retrieval transport are implemented and measured.
Implementation:
`tools/artifact-generator/`, `crates/converter-artifact/`,
`crates/zigbee-apiarist/tests/differential_zcl_pinned.rs`.
Research baseline: [`converter-artifact-research.md`](converter-artifact-research.md).

The dispatcher wiring is opt-in (`ConverterDispatcher::start_lazy`); the
default bridge path is unchanged. No live coordinator/MQTT/HA operation was
executed, and no real artifact mirror was contacted.

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
  prepared-definition equality and behavioral differential. Behavioral
  verification is mandatory for publication: absent/skipped verification
  falls back to the original module. `--allow-unverified-split 1` is an
  explicit development-only escape hatch; production generation does not use
  it.

CLI and reproduction commands are in
[`../tools/artifact-generator/README.md`](../tools/artifact-generator/README.md).

### Determinism

Two independent runs produced identical `manifest.json` and
`index.z2idx1` hashes:

```text
manifest  3d96a3c6...819050
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
| compact JSON | 1,454,324 | 86–124 ms | 7.8 MB | 0 |
| `z2idx1` | 927,458 | 68–86 ms | 5.4–6.1 MB | 0 |

`crates/converter-artifact/src/bin/resolver_diff.rs` compares both formats
against the same oracle; both report **0/11,562** failures. `z2idx1` is chosen:
34% smaller, lower memory, equal or faster parse on the target.

The earlier `models-index` path for the real `TS0601/_TZE200_locansqn` device
loaded 73 files/4.16 MB/8.17 s. The resolver returns the pinned result without
loading a device module; with white-label overlay it returns `TH01Z` (Tuya),
and the underlying definition is `nous.js`.

## 2a. Why the candidate count is 6023, not 8469

The earlier prototype index had **8469** candidate entries because it indexed
white-label fingerprints as lookup keys and did not deduplicate candidates.
The production index follows the pinned upstream index exactly:

- pinned `models-index.json`: **5069 keys, 6023 candidate references**;
- each reference is unique per `(key, file, definition index)` in upstream
  (`addToLookup` deduplicates via `some(...)`);
- **998** white-label fingerprint references are **not** lookup keys upstream:
  they are stored in the definition for the post-resolution white-label
  overlay;
- the remaining prototype difference is **1448 duplicate references** caused
  by multiple fingerprints/zigbeeModels for the same definition under the same
  key.

The generator consumes the pinned `models-index.json` order directly, maps each
`[file, index]` to one definition id and stores the resulting id array without
reordering or adding candidates. The resolver differential (11,562 profiles,
0 failures) includes priority ties and verifies the resulting choice against
Node `findByDevice`; candidate order is therefore the upstream order, not a
first-candidate heuristic.

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

## 3a. Lazy loader and versioned cache

`converter-artifact` now includes:

- `read_database`: NDJSON `database.db` reader producing device profiles;
- `LazyLoader::startup`: resolves only paired records through the index, maps
  the selected definition file to a manifest module and loads only that module
  plus its module dependency closure;
- `LazyLoader::pair`: resolves the new fingerprint and runs the same bounded
  closure load on a background thread; existing devices use the same verified
  cache entries and are not blocked;
- `ModuleCache`: `<cache>/<artifactId>/generations/<gen>` immutable generations,
  `staging/<gen>` transactions and an atomically replaced `active.json`
  pointer; `lkg.json` keeps the last-known-good generation for rollback.
  Commit order is: fsync staged files, fsync staging dir, rename into
  `generations/`, fsync `generations/`, fsync+rename `lkg.json`, fsync+rename
  `active.json`, fsync the cache root. Every fsync error is returned; a
  failure after the `active.json` rename reports "activated but not durably
  synced" and keeps the new generation, because the pointer already names it;
- explicit `ModuleState`: `Missing`, `Pending`, `Downloading`, `Verified`,
  `Ready`, `Failed(reason)`;
- budgets: per-module download, peak on-disk transaction footprint
  (`current + staging copy + downloads`, used for `max_cache_bytes`) and
  in-memory fetch-chunk cap. `SpaceProbe` is asked a different question — how
  many *new* bytes may be allocated — so it is compared with
  `staging copy + downloads`, never with the peak: bytes already occupied by
  the cache are not free space, and subtracting them twice would reject
  transactions that fit a nearly full overlay;
- offline mode: `ensure(..., allow_network=false)` succeeds only from verified
  cache; a miss is an explicit error and never triggers a silent fetch.

The trusted manifest is loaded from the installed artifact; cache roots are
artifact-id/version scoped. A module is checked against its manifest sha256,
size, module id and artifact compatibility before activation. Network transport
is an injected `ModuleFetcher`; `converter::http::HttpsFetcher` is the
production implementation, and tests use a file-backed fetcher plus explicit
failure fetchers.

Automatic tests: `cache_manager.rs` (closure/offline, hash mismatch, partial
download, network failure, insufficient-space semantics on a nearly full
overlay, rollback, ABI, peak/transaction budget, memory budget, 8-way
single-flight, aborted transaction, cancelled (panicking) fetch and lock
recovery, crash recovery from orphan staging and a corrupt active pointer,
commit I/O failure keeping the previous generation active) and
`lazy_loader.rs` (paired startup, real nous/IKEA/Tuya definitions, offline
restart, per-device independence with one missing module, concurrent startup
fetching each shared module once, runtime generation switch/retention and a
failed runtime materialization preserving the active generation).
`https_fetcher.rs` covers TLS against a local rustls server, Content-Length,
chunked transfer, same-host and rejected off-host redirects, truncated bodies,
timeouts, the response-size cap, HTTP errors and SHA-256 mismatch refusal; one
ignored test validates the public root store, redirects and chunked transfer
against public HTTPS mirrors.

Routerich synthetic measurement (20 paired devices resolving to one module,
isolated `/tmp` cache, no production overlay writes):

```text
loader cold start   23.9 ms
modules loaded      nous
network bytes       25,780
cache after start   25,780 B
cache after pair    315,722 B (nous + tuya-6 + tuya-common)
baseline startup    p50 22.0 ms / p95 35.9 ms
under pairing       p50 23.7 ms / p95 49.6 ms

full bridge cold start to 20 device definitions ready (real quickjs
prepareDefinition, release aarch64, dry-run transport/lifecycle stage):
  loader startup     2,258.8 ms
  prepare all          596.3 ms
  total              2,882.6 ms
```

The pairing fetch downloads only the selected `tuya` fragment closure, not the
full device catalog; p95 degradation is measured, not assumed.

Single-flight and pairing impact on the same router (artificial fetch latency to
make network time visible; 3 distinct closures, each module fetched exactly
once in both runs):

```text
fetch delay 30 ms    sequential 244.8 ms   concurrent 226.5 ms
fetch delay 200 ms   sequential 765.7 ms   concurrent 718.0 ms
```

Concurrent different closures are serialized by the global `ensure_lock`, so the
elapsed time tracks the sequential sum. Pairing is sequential upstream, startup
downloads are exceptional and the cached fast path takes no lock, so no
per-module lock is added without a real bottleneck.

Report-path latency while a new Tuya fragment is downloaded (120 ms per module)
and prepared on the shared JS worker (300 report samples, 52 overlapping the
pairing window):

```text
pairing total        1,758 ms
baseline p50/p95     2.50 / 6.02 ms
during   p50/p95     2.66 / 6.10 ms
during   max           847 ms (one report queued behind prepareDefinition)
```

The download runs on blocking threads and does not stall reports; p50/p95 are
unchanged. One report waits for the single JS worker's `prepareDefinition`
job. That head-of-line behavior is inherent to the single-threaded JS realm and
matches the upstream Node event loop; lowering it needs a scheduler priority
feature, which is not added without a second, repeated stall measurement.

## 3b. Bridge wiring and cache consistency

- `ConverterDispatcher::start_lazy` calls `LazyLoader::startup` from the
  existing bridge database, materializes only the selected closures into a
  runtime package root, then starts the existing quickjs-oxide dispatcher. No
  second loader or dispatcher is created.
- `ConverterDispatcher::prepare_with_file` makes the selected artifact module
  file explicit, so runtime bootstrap does not fall back to an unselected
  catalog module.
- `pair_device` loads the new device's closure through the same cache and
  prepares its definition through the same runtime. Existing devices continue
  using their already-verified modules.
- Runtime materialization is versioned too: `materialize_runtime_package`
  writes a complete package into `<base>/generations/.staging-<gen>`, fsyncs
  it, renames it to `<base>/generations/<gen>` and only then atomically replaces
  `<base>/active.json`. The base directory and the previously active generation
  are never deleted before a new generation is complete; the previous
  generation is retained for a JS worker that is still shutting down and is
  pruned by the next start. Pairing writes additional modules only into the
  active generation, through temp+rename (or an immutable inode hard link), so
  a running worker never sees a partial file. Cache and artifact files are
  hard-linked when on the same filesystem and copied+fsynced otherwise.
- ModuleCache now uses immutable generations plus an atomically replaced
  `active.json` pointer and `lkg.json` rollback pointer. Staging is never
  active. `open()` ignores orphan staging/generations, falls back to LKG when
  the active pointer/generation is invalid, and cleans stale staging. A crash
  before the pointer swap leaves the old generation active; a crash after it
  leaves a complete new generation.
- `ensure` has a cached fast path (no lock for already-ready devices), a global
  activation/budget lock for single-flight behavior, and per-module fetch
  concurrency bounded by `max_concurrent_downloads`. Concurrent ensures never
  fetch the same module twice or activate overlapping generations. Different
  closures downloaded concurrently are serialized by the global lock; the
  measurement in §6 records the cost of that serialization. It is kept
  because pairing is sequential upstream and startup downloads are exceptional;
  no per-module lock was added without evidence of a real bottleneck.
- `converter::http::HttpsFetcher` is the production-shaped fetcher: HTTPS only,
  host fixed by the trusted `modules.retrieval` base (which must end in `/`),
  no device URLs, response cap, connect/read/write timeouts, redirect limit and
  same-host redirects. SHA-256/size verification remains in ModuleCache before
  activation.
- The fetcher is injected; the offline bridge tests never construct it, and no
  real network request was made. `modules.retrieval` is now emitted by the
  generator (`--retrieval-url`) and recorded in the manifest.
- Cached/existing devices take the lock-free fast path, so a pairing download
  holding the activation lock does not delay them; download work runs on
  `spawn_blocking` threads and never on the Tokio reactor or the JS worker.
- A paired device whose module is absent at offline startup is reported as a
  per-device failure and stays definition-less; other paired devices keep their
  verified modules and work offline. A later `pair_device`/lazy restart retries
  the closure; there is no unbounded background retry loop.

## 3c. Real Endpoint transport and lifecycle

- `DryRunEndpointTransport` implements `ConverterTransport` on top of the real
  `Endpoint` model. `command`, `commandResponse`, `write`, `read` and
  `configureReporting` go through the existing endpoint methods; a per-call
  `DryRunHook` inside `Endpoint::send_request` captures the exact frame bytes
  that the adapter would send and returns a synthetic `readRsp` built from
  seeded values. Nothing is transmitted and no coordinator is needed.
- Command options are preserved end-to-end: the runtime endpoint shim now
  forwards `options` (`manufacturerCode`, `disableDefaultResponse`,
  `direction`, `srcEndpoint`, `transactionSequenceNumber`, `reservedBits`,
  `timeout`, `writeUndiv`) and the endpoint applies the same defaults as
  upstream. Manufacturer-specific clusters/attributes and device custom
  clusters are resolved by the existing `Endpoint::get_cluster` path.
- `bind` is a ZDO operation: in dry-run it is recorded as an intent (target id
  and type) without transmitting; production will call `Endpoint::bind`.
- Lifecycle: `Zigbee::schedule_converter_pairing` runs after a successful
  device interview. It deduplicates by IEEE, uses one sequential background
  worker, skips devices that already have a definition (existing devices are
  prepared first by `prepare_converter_definitions`) and registers the
  prepared definition in the existing registry.
- Opt-in startup: `converter::startup::install_from_env` is invoked before
  `controller.start()`. It is disabled unless `Z2M_CONVERTER_LAZY=1` and
  requires `Z2M_CONVERTER_ARTIFACT_DIR` and `Z2M_CONVERTER_HERDSMAN_DIR`;
  cache/runtime default under the data directory. Only
  `Z2M_CONVERTER_TRANSPORT=dry-run` is accepted in this stage; any other value
  disables lazy mode with an explicit error (never a silent fallback). No
  settings schema or `configuration.yaml` key is changed.
- Reports during pairing are queued by the existing bounded runtime channels
  (admission + `max_pending_events`), not dropped silently. The lifecycle test
  submits 120 reports while a new Tuya device is downloaded and prepared and
  asserts every report completes; a full queue surfaces `Backpressure`
  explicitly.

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

- **415 ZCL cases** (206 encode, 209 decode) across all 129 clusters,
  commands/commandsResponse, manufacturer-specific cluster data, global
  commands and corrupted/truncated buffers;
- **60 ZDO constants** with names/IDs.

The two former "mismatches" were an oracle bug, not a codec bug: greenPower and
telecommunicationsVoiceOverZigbee both have two commands with the same ID
(`commissioningNotification`/`sinkCommissioningMode` = 4 and
`establishmentRequest`/`controlResponse` = 0). The generator re-encoded a frame
created from one command and then asserted that the *other* command could
decode the bytes. Pinned upstream throws `INVALID_FIELD` on those buffers as
well (verified directly with Node). `generate-zcl-pinned.mjs` now asks upstream
to decode before emitting a round-trip case and records such buffers with
`expectedError: true` + `upstreamDecodeError: true`; the Rust decoder returns
the same error. `openDefects` is therefore **0** for this corpus.

`differential_frame_options.rs` adds 10 pinned cases for command options:
default-response flag, direction, TSN, reserved bits, manufacturer codes
(header and write payload), read, configReport, readRsp and a custom cluster.
The dry-run transport tests compare the endpoint-produced bytes with the same
oracle and cover custom clusters, manufacturer-specific commands and `bind`.
Unknown inbound clusters follow the existing raw-diagnostic path; unknown
outbound clusters remain a documented next-stage error path (no guessed
encoding).

## 6. Measured artifact

| Component | Bytes |
|---|---:|
| `z2m-rs` aarch64 musl | 23,455,568 raw / 8,036,565 gzip (unstripped); 16,610,936 raw / 6,980,370 gzip stripped |
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
- The greenPower/telecom duplicate-ID receive cases are asserted as errors on
  both sides; no known codec mismatch remains in the pinned corpus.
- ZDO frame corpus currently covers constants plus existing wire/zdo suites; a
  full ZDO request/response payload oracle remains next-stage work.
- The lazy loader/cache is wired into `controller.start()` through
  `install_from_env` (env-gated) and into interview-driven pairing through
  `Zigbee::schedule_converter_pairing`. The default remains the existing
  package-root flow until the artifact is installed and the env switch is set.
- `HttpsFetcher` is covered by a local rustls TLS server (TLS, chunked,
  redirects, truncation, timeout, size cap, 404, hash mismatch) and by a manual
  run against public HTTPS mirrors (`example.com`, `httpbin.org` redirect and
  chunked stream). It has not been exercised against a real artifact mirror,
  because no artifact mirror exists yet.
- The router overlay was 99% full (568 KB free), so the runtime/cache
  measurements use `/tmp` (tmpfs) with same-filesystem hard links. A
  flash-resident runtime measurement needs the package migration.
- The full artifact generator/oracle commands remain explicit build/research
  steps.
- No OpenWrt package migration, no live coordinator/MQTT/HA run and no HIL.

## 8. GO / NO-GO

**GO** for the real Endpoint transport & integration validation stage:

1. `z2idx1` resolver is 0-diff on the full oracle (11,562 cases) and golden
   parity is 36/36;
2. the two former ZCL "mismatches" are resolved as oracle bugs: upstream
   rejects the same duplicate-ID buffers, the generator now requires an
   upstream round-trip and asserts both sides error (`openDefects: 0`);
3. the frame-option differential (10 cases: default-response, direction, TSN,
   reserved bits, manufacturer code, read/write/configReport/readRsp, custom
   cluster) matches pinned upstream;
4. `DryRunEndpointTransport` builds frames through the real Endpoint/ZCL model,
   forwards command options, serves reads from seeded values and records bind
   intents without transmitting anything;
5. lazy loading is wired into `controller.start()` (env-gated) and into
   interview-driven pairing with a deduplicated, bounded, single-worker queue;
6. 120 reports complete while a new Tuya device is downloaded/prepared in the
   background; queues are the existing bounded runtime channels and overflow is
   explicit backpressure;
7. PERF-001 (≤0.85 s single-report stall during manual pairing) is accepted as
   low-priority debt in `docs/backlog.md`;
8. Routerich criteria hold: 20 devices ready in 2.88 s, single-flight and
   pairing impact within the previous envelope.

**GO for the agreed coordinator-only HIL**, subject to `docs/hil-plan.md`:
dedicated coordinator/maintenance window, committed overlay/package migration,
recorded binary/artifact hashes, no `StartCommissioning`/restore, rollback to
the Node.js service ready.

**NO-GO for production enablement** until:

1. the real (transmitting) Endpoint transport replaces dry-run and passes the
   HIL plan; this build records frames only;
2. an artifact mirror exists and `HttpsFetcher` is validated against it with a
   real manifest (public HTTPS mirrors and a local TLS server are covered);
3. the overlay migration and procd service are exercised on the router;
4. the ZDO frame corpus is expanded with request/response payloads;
5. HIL evidence is captured for the operations listed in `docs/hil-plan.md`.
