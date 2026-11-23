# Converter artifact research and implementation plan

Status: research complete, implementation not started.
Scope: OpenWrt artifact, lazy device loading, pairing, tuya split, ZCL/ZDO
audit and offline differential plan.
Hardware: Routerich AX3000 v1, OpenWrt 24.10.8, aarch64.
Pinned upstream: zigbee2mqtt `2.14.3` (`3c841fdf`), zigbee-herdsman `11.2.0`,
zigbee-herdsman-converters `26.125.0`, Node `22.23.4`.
The Node service was left running; no coordinator/MQTT/HA operation was
performed and `ref/configuration.yaml` was not changed.

Raw measurements: `tools/bench/results/artifact-study/`.
Scripts: `tools/artifact-study/`.

## 1. Executive summary

1. The current converter directory is **26 MB** on disk including maps/types;
   runtime JavaScript is **11.65 MB** (9.23 MB device files, 2.19 MB shared
   libraries, 0.23 MB common converters) across **401** device JS files. Only 38
   shared files (**2.52 MB** raw, **428 KB** gzip) are needed by every device;
   maps/types add another ~9.7 MB and are not needed at runtime.
2. The current `models-index.json` is **199 KB / 5069 keys**, but it only maps a
   model key to `[file, index]`. Resolving the actual paired Tuya device
   (`TS0601`, `_TZE200_locansqn`) through upstream loaded **73 files, 4.16 MB,
   in 8.17 s** before it found `nous.js` (`SZ-T04`). A fingerprint-aware index
   resolves the same definition without loading any device module.
3. A rich candidate index (model/vendor/zigbeeModel/fingerprint/priority/file/
   index) is **892 KB JSON, 131 KB gzip**; it contains **4619 definitions,
   5077 lookup keys, 8469 candidate entries**. A binary index is expected to be
   well below that and is the recommended package format.
4. Node resolution and Rust runtime cold starts are dominated by shared
   library/package load: Node root **3.37 s** and shared modules **4.04 s**;
   Rust worker startup **2.13–2.19 s**. First device preparation after startup
   was **0.53 s** (`nous.js`) and **0.77 s** (`ikea.js`) in Rust.
5. The Rust ZCL catalog is definition-equivalent to
   pinned upstream: **129 clusters, 1354 attributes, 254 commands, 171 command
   responses**, with **0 differences** in names, IDs, types, parameters,
   conditions, manufacturer codes and required flags. The existing differential
   fixture only has **7 cases**, so coverage—not catalog data—is the gap.
6. The ZDO audit found a real defect: five request constants were offset from
   upstream (`RETRIEVE_AUTHENTICATION_TOKEN_REQUEST` 0x0042→0x0041,
   `GET_AUTHENTICATION_LEVEL_REQUEST` 0x0044→0x0042,
   `SET_CONFIGURATION_REQUEST` 0x0049→0x0043,
   `GET_CONFIGURATION_REQUEST` 0x004a→0x0044,
   `START_KEY_UPDATE_REQUEST` 0x004b→0x0045). They are fixed in this change
   with a regression test; an expanded ZDO differential is still required.
7. Automatic split of `tuya.js` is feasible. It has **571 definitions**,
   **1.368 MB** of definition text and a **106 KB** preamble. Eight fragments of
   150–182 KB plus one 107 KB shared preamble load in **3.7–4.3 s** on Routerich
   versus **8.0 s** for the whole file. Structural comparison and
   `prepareDefinition` equality passed **571/571** with **0 differences**.
8. The Rust binary dominates the opkg: **18 MB raw / 6.07 MB gzip**. A staged
   package with binary + shared JS + rich index is **6.68 MB gzip**. The current
   Node opkg is 12.24 MB compressed / 53.6 MB installed. The router overlay has
   only **1.1 MB free (98% used)**, so side-by-side installation is impossible;
   migration must remove/replace the Node package or use external storage.

## 2. Package and dependency graph

Measured in `.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters`:

| Area | Files | Bytes |
|---|---:|---:|
| `dist/devices/*.js` | 401 | 9,232,586 |
| `dist/lib/*.js` | 33 | 2,189,766 |
| `dist/converters/*.js` | 3 | 225,267 |
| `dist/index.js` | 1 | 28 KB |
| `dist/models-index.json` | 1 | 198,945 |
| source maps | — | 9.1 MB (not needed) |
| `.d.ts` | — | 637 KB (not needed) |

Dependency closure of all non-device modules: **39 files / 2.65 MB** including
the current index; **38 files / 2.52 MB** without it. The union of all device
closures is 11.6 MB because of shared modules; the complete device directory is
21 MB.

Per-device closure (includes shared libraries, so marginal device cost is much
smaller):

| Device module | Closure bytes | Files | Marginal file |
|---|---:|---:|---:|
| `nous.js` | 1,195,777 | 15 | 25,780 |
| `ikea.js` | 1,049,241 | 15 | 55,827 |
| `danfoss.js` | 1,033,450 | 14 | 91,523 |
| `lumi.js` | 1,689,143 | 15 | 300,196 |
| `tuya.js` | 2,673,631 | 16 | 1,475,081 |

Common relative imports from device files: `lib/modernExtend` (305),
`lib/exposes` (255), `converters/fromZigbee` (198), `lib/reporting` (167),
`converters/toZigbee` (128), `lib/utils` (98), `lib/tuya` (81). External
requires are only `zigbee-herdsman`, Node builtins, `semver`,
`fast-deep-equal`, `iconv-lite`; all are already shimmed or host-provided.

## 3. Why the current index is insufficient

`models-index.json` is `Record<zigbeeModel|fingerprint.modelID, [file, index][]>`.
`findDefinition` loads every file referenced by the key and filters candidates
in JS. For a real paired device this is expensive:

```text
TS0601 / _TZE200_locansqn:
  candidates: 1394 index entries across zemismart/tuya/nous/...
  loaded:     73 files, 4,259,138 bytes
  time:       8,170 ms (Routerich, Node, cold process)
  result:     SZ-T04 (Nous), file nous.js, index 3
```

A rich index entry for the same key contains the matching fingerprint
(`modelID: "TS0601"`, `manufacturerName: "_TZE200_locansqn"`), so the resolver
can select `nous.js`/3 without loading any device file. This is the required
format for pairing and normal startup.

Rich index prototype statistics:

| Metric | Value |
|---|---:|
| definitions | 4,619 |
| lookup keys | 5,077 |
| candidate entries | 8,469 |
| files | 400 |
| fingerprint entries | 3,700 |
| white-label entries | 1,247 |
| largest candidate set | `ts0601` (1,394) |
| JSON bytes | 892,107 |
| JSON gzip | 130,810 |
| Rust `serde_json` parse, Routerich | 134.6 ms (200 iterations best) |
| Node parse, Routerich | 253.5 ms |

A binary index with a string table and varint records is the target; expected
raw size is 250–400 KB and parse time should be <50 ms on Routerich. The
current 199 KB index is 29.6 ms in Rust, so this is a modest cold-start
trade-off for eliminating multi-second resolution.

## 4. Cold start and cache

### 4.1 Cold process measurements (Routerich, Node)

| Workload | Elapsed | Loaded files | Loaded bytes | RSS |
|---|---:|---:|---:|---:|
| shared modules + index | 4,044 ms | 13 | 1,124,094 | 48.3 MB |
| package root (`dist/index.js`) | 3,369 ms | 18 | 1,301,255 | 48.3 MB |
| `tuya.js` | 6,277 ms | 16 | 2,673,631 | 60.1 MB |
| `ikea.js` | 3,764 ms | 15 | 1,049,241 | 47.9 MB |
| `zemismart.js` | 3,673 ms | 15 | 1,234,842 | 47.7 MB |
| `danfoss.js` | 3,143 ms | 14 | 1,033,450 | 47.1 MB |
| `lumi.js` | 4,386 ms | 15 | 1,689,143 | 50.7 MB |
| external `L101Ze_z2m.js` | 3,091 ms | 18 | 1,301,255 | 48.9 MB |

### 4.2 Rust runtime (Routerich)

| Workload | Worker startup | Prepare total | Prepare eval | JS heap |
|---|---:|---:|---:|---:|
| `nous.js` | 2,129 ms | 526 ms | 525 ms | 5.63 MB |
| `ikea.js` | 2,188 ms | 768 ms | 768 ms | 5.63 MB |

Worker startup includes the package root/shared modules. The first device
module is the main lazy-load cost; shared library load is paid once per realm.

### 4.3 Actual paired-device cache

The live database contains four NDJSON records: coordinator, `TS0601`
(`_TZE200_locansqn`), and two `L101Ze.v2*` LYTKO devices. The Tuya device
resolves to `nous.js`; the LYTKO devices use a local external converter.

| Cache item | Bytes |
|---|---:|
| `nous.js` | 25,780 |
| external `L101Ze_z2m.js` | 28,750 |
| tuya split common + one fragment (if a Tuya device needs it) | 107,440 + ~175,000 |
| staged cache (nous + external + tuya common/fragment) | 337,043 raw / 60 KB gzip |

Without the split, a Tuya device would cache the whole `tuya.js`:
1,475,081 raw / 204,329 gzip. With split, the cache stays ~337 KB raw.

## 5. Opkg size estimate

Staged data archive (z2m-rs binary + shared JS closure + rich index + manifest,
no device modules):

| Component | Raw | gzip |
|---|---:|---:|
| `z2m-rs` aarch64 musl | 18 MB | 6,069,341 |
| shared JS closure | 2,519,040 | 428,247 |
| rich index JSON prototype | 892,276 | 130,866 |
| manifest/control | ~10 KB | ~2 KB |
| **data.tar.gz estimate** | — | **6.68 MB** |

The current Node package reports `Size: 12,239,632` and installs **53.6 MB**;
the overlay has **54.3 MB total, 50.4 MB used, 1.1 MB free**. Side-by-side
installation is not possible on this router. The migration plan must either
remove `node-zigbee2mqtt` first or stage the artifact on external storage; this
is a deployment decision, not a runtime design change. The Rust package is
roughly half the current opkg compressed size.

## 6. Artifact manifest and index format

Compatible example: `docs/converter-artifact-manifest.example.json`.

Required manifest properties:

- `manifestSchema`, `artifactId`, `generatedAt`;
- upstream versions and the exact z2m commit;
- host ABI, engine/version, bootstrap/ZCL catalog hashes, required features;
- index file/format/sha256/compressed hash and entry counts;
- shared tree file count/bytes/hash;
- module entries with file, sha256, bytes, definition count and dependencies;
- fragment metadata (common module, parts, verified definition count);
- cache layout, pinned/offline policy, hash/ABI/feature verification;
- explicit refusal of unknown outbound clusters and the raw-diagnostic policy
  for unknown inbound clusters.

Recommended binary index layout (`z2idx1`):

```text
header: magic "Z2IDX1", u16 version, u16 flags, u32 counts
string table: u32 count, then (u32 offset, u32 length) + UTF-8 bytes
files: u32 count, file record {string_id, module_id, size, sha256[32]}
definitions: u32 count, record {
  u16 file_id, u16 def_index,
  string_id model, string_id vendor,
  u16 zigbee_model_count, string_id[] zigbee_models,
  u16 fingerprint_count, fingerprint[],
  string_id version
}
keys: u32 count, map {string_id normalized_key, u32 candidate_count, u32 definition_id[]}
```

Fingerprint records carry the exact upstream comparison fields
(`applicationVersion`, `manufacturerID`, `type`, `dateCode`,
`hardwareVersion`, `manufacturerName`, `modelID`, `powerSource`,
`softwareBuildID`, `stackVersion`, `priority`) plus white-label model/vendor.
Resolution order must match `index.js`:

1. normalize key (lowercase, trim, `null` for missing);
2. match fingerprints, taking the highest priority;
3. fall back to exact/normalized `zigbeeModel` candidates;
4. apply white-label model/vendor overlay;
5. only then load the selected module/fragment.

## 7. tuya.js split feasibility

`tuya.js` structure:

| Metric | Value |
|---|---:|
| file bytes | 1,474,732 |
| preamble bytes | 106,337 |
| `exports.definitions` array bytes | 1,368,361 |
| definitions | 571 |
| definition bytes | 1,364,744 |
| smallest/median/largest definition | 177 / 1,531 / 15,161 bytes |
| top-level binding names in preamble | 71 |

Prototype: one `tuya-common.js` (preamble + exported bindings, 107.4 KB) and
eight fragment modules of 150–182 KB, each requiring the common module and
exporting a contiguous subset. Exact element text is preserved; fragments are
mapped by `(file, index)` in the manifest.

Verification:

- raw structural comparison (`model`, vendor, zigbeeModel, fingerprint,
  whiteLabel, from/to converters, exposes kind, configure/onEvent, options):
  **571/571 equal, 0 differences**;
- `prepareDefinition` comparison: **571/571 equal, 0 differences**;
- Node cold load, Routerich: original **8,036 ms**, fragments **3,744–4,258 ms**;
  loaded bytes 2.67 MB → 1.46–1.48 MB.

Conclusion: split is allowed and functionally equivalent for this file. The
production generator must use a real parser (not regex) and must rerun the
571-definition equivalence check before publishing the artifact. If the check
fails, publish the whole file for that module.

## 8. ZCL / ZDO audit

### ZCL

Compared pinned upstream `zspec/zcl` with
`crates/zigbee-apiarist/src/zspec/zcl/definition/clusters.json` by name, ID,
attribute ID/type/manufacturerCode/conditions/max and command
ID/response/required/parameters:

```text
upstream clusters: 129, rust: 129
missing/extra clusters: 0
attributes: 1354, commands: 254, commandsResponse: 171
metadata differences: 0
```

Manufacturer-specific support: upstream has one cluster with
`manufacturerCode` (`manuSpecificAmazonWWAH`, 0xFC8F/4631) plus `manuSpecific*`
clusters. Rust stores `manufacturerCode` and supports device
`add_custom_cluster` / `CustomClusters`; custom-cluster lookup semantics need
dedicated differential cases because catalog equality does not prove runtime
selection.

### ZDO

Upstream has 60 `ClusterId` constants; Rust has 60 names, but five request
values are wrong:

| Constant | Upstream | Rust |
|---|---:|---:|
| `RETRIEVE_AUTHENTICATION_TOKEN_REQUEST` | 0x0041 | 0x0042 |
| `GET_AUTHENTICATION_LEVEL_REQUEST` | 0x0042 | 0x0044 |
| `SET_CONFIGURATION_REQUEST` | 0x0043 | 0x0049 |
| `GET_CONFIGURATION_REQUEST` | 0x0044 | 0x004a |
| `START_KEY_UPDATE_REQUEST` | 0x0045 | 0x004b |

`DECOMMISSION_REQUEST` (0x0046) and all response constants checked are correct.
The wrong values affected key-negotiation/authentication ZDO frames; the
constants are fixed now and covered by a unit test. The external ZDO differential
is still part of Phase 5.

### Unknown frames

Incoming unknown/unparseable frames are currently logged and emitted as
`type: "raw"` with the original payload (controller path), which is the desired
diagnostic behaviour. Outgoing unknown clusters still go through
`Utils::get_cluster`, which panics for names outside the catalog. The
differential implementation must return an explicit error for unknown outbound
clusters; never encode a guessed payload.

## 9. Implementation plan

### Phase 1 — artifact generator (build-time Node, no runtime changes)

1. Pin z2m/zh/converters/Node revisions; generate the rich binary index and
   manifest; hash shared tree, index, modules and manifest.
2. Split `tuya.js` with a parser, emit common + fragments, run raw and
   `prepareDefinition` equivalence; fall back to the whole file on any failure.
3. Add optional split for other files above a size threshold only after the
   same equivalence gate.
4. Emit a compatibility descriptor (host ABI, engine, ZCL catalog hash,
   required features).

Acceptance: deterministic artifact hash; 571/571 tuya equality; index resolver
matches upstream `findDefinition` for every candidate key/fingerprint in the
pinned corpus; no maps/types in runtime data.

### Phase 2 — Rust index and resolver

1. Add `z2idx1` reader with bounds checking and a hard maximum size.
2. Implement candidate resolution matching upstream fingerprint priority,
   normalized keys and white labels.
3. Replace the current `model → first file` lookup path with the resolver.
4. Golden tests: full 5077-key resolution table generated from Node and the
   actual paired device profile.

Acceptance: 0 resolver differences against upstream; index parse <50 ms on
Routerich; malformed index fails closed with a diagnostic.

### Phase 3 — lazy device loader and cache

1. Resolve only paired devices at startup; never load all device modules.
2. Load one selected module/fragment per device; cache immutable, hash-checked
   modules under `/var/lib/z2m-converters/cache/<artifactId>`.
3. Verify sha256, host ABI and required features before evaluation; keep the
   last-known-good module on failure.
4. Paired modules are pinned and work offline; network is only required for a
   missing module.
5. Add the cache root to the runtime file sandbox (read-only after install).

Acceptance: cold start bytes/time scale with paired devices; paired offline
restart; corrupt/mismatched cache rejected without replacing the previous
module.

### Phase 4 — pairing and priority

1. Pairing remains user-triggered; use the rich index for the new device.
2. Fetch/prepare missing modules in a low-priority background queue.
3. Runtime scheduling gets two classes: existing-device work first, pairing
   warmup/yieldable second; no unbounded waits, no mass exposes generation.
4. Persist the selected module id/file/index in device metadata or derive it
   from the database + index deterministically.

Acceptance: existing reports/sets unaffected while a pairing fetch runs;
pairing timeout is explicit; rollback is atomic.

### Phase 5 — offline ZCL/ZDO differential

1. Add a pinned Node oracle for `Frame.create/toBuffer/fromBuffer` and ZDO
   encode/decode.
2. Cover all 129 clusters with default payloads, manufacturer-specific
   clusters, custom-cluster injection, unknown inbound and unknown outbound.
3. Compare bytes and parsed JSON; test `add_custom_cluster` replacement,
   manufacturer-code selection and invalid/truncated buffers.
4. Verify the corrected ZDO constants and fix any codec differences found.
5. Unknown outbound returns an explicit error; unknown inbound follows the
   logged raw diagnostic path.

Acceptance: byte-for-byte parity for the generated corpus; no panics on
malformed input; unknown-outbound test returns an error before any adapter call.

### Phase 6 — measurements and deployment

1. Re-measure opkg size, cache size, cold start, paired-device startup and
   first-command latency on Routerich.
2. Resolve the overlay constraint (remove Node package or external storage)
   with the user before installation.
3. Keep golden parity and no-coordinator tests green; HIL only after explicit
   approval.

## 10. Risks and decisions

- **Overlay capacity:** 1.1 MB free makes side-by-side install impossible; this
  is a deployment blocker, not a code blocker.
- **Binary size:** 18 MB raw / 6.07 MB gzip; quickjs dominates. Stripping and
  LTO should be measured before packaging; no C linkage is allowed.
- **Index format:** rich JSON is 892 KB and 135 ms parse; binary format is
  required to keep cold start bounded.
- **Split safety:** automatic split is only accepted with the full prepared
  equivalence gate; otherwise ship the whole module.
- **Model index coverage:** 5077 keys vs 4619 definitions; fingerprint-only,
  white-label and null-key entries must be preserved exactly.
- **External converters:** the paired LYTKO file is local and 28.7 KB; it must
  remain outside the network artifact and be included in the pinned cache.
- **ZCL/ZDO:** catalog data is complete, but differential coverage is only 7
  cases and ZDO has five incorrect constants.

## 11. GO / NO-GO

**GO** for Phase 1–2 (artifact generator + rich binary index + resolver) and
Phase 5 test scaffolding; these can be developed fully offline and are required
before any lazy loading.

**NO-GO for enabling lazy loading in the production bridge** until:

1. the rich index resolver matches upstream for the full candidate corpus;
2. the expanded ZCL/ZDO differential is green (the five ZDO constants are
   already fixed);
3. cache hash/ABI/rollback tests pass;
4. pairing priority and offline-paired behavior are measured on Routerich;
5. the OpenWrt overlay migration (remove Node or external storage) is agreed.

**NO-GO for HIL** until an explicit separate approval; this research did not
touch the physical coordinator.
