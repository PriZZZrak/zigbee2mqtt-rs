# Upstream Changes

## 2026-11-04T09:08:33.742736+00:00 — planning audit refresh

Baseline remains app `344b6e03b43644c97154a72ca9eae17193c553a2`, herdsman
`3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`, converters
`9dabbe4e343573c5a3070605efd843a1b3a16058`. **DRIFT_DETECTED**.
All five final fetches succeeded; initial documentation fetch had timed out.
All reference working trees remained clean. Source pins/integrities/check times:
[upstream-lock.json](upstream-lock.json). Findings/tests:
[AUDIT-20261104](evidence/AUDIT-20261104.md).

Historical entries below are retained as history, not current evidence: their
herdsman/converter SHA-version pairings and old completeness claims are not
authoritative. Existing Windfront baseline 2.15.1 conflicts with the app's actual
2.15.0 dependency. Classic frontend 0.9.21 was omitted from tracking. This audit
records both gaps rather than silently moving the compatibility baseline.

| Delta | Old → final head | Impact and disposition |
|---|---|---|
| APP-20261104 | [344b6e03…3c841fdf](https://github.com/Koenkk/zigbee2mqtt/compare/344b6e03b43644c97154a72ca9eae17193c553a2...3c841fdfd66c92239921ef84fde8f039c64e1662) | Latest head unchanged since previous check. Pending dependency/availability/HA review from prior entry; 33 changed files. No full drift acceptance. |
| HERDSMAN-20261104 | [c1a54e3d…eb80ba1f](https://github.com/Koenkk/zigbee-herdsman/compare/c1a54e3d52858fa0055aeaf1cf74c3dac65e9811...eb80ba1f1cb339071b0557fabd18fa856d4ce36a) | Eight commits since prior recorded latest; now 11.2.1. Behavioral diffs below inspected; reference execution/backport decision pending. |
| CONVERTERS-20261104 | [c18c32d9…36710028](https://github.com/Koenkk/zigbee-herdsman-converters/compare/c18c32d94c77f683fa768618d1fd32b348fdd7b9...367100282578111770f1c8ab0e38805d9ec581d0) | 65 commits since prior recorded latest; now 26.126.0. Catalog changes remain largely untriaged, not verified Rust support. |
| UI-20261104 | [353c7d89…b3222692](https://github.com/Nerivec/zigbee2mqtt-windfront/compare/353c7d89f1d8bf005db435c4ec809e6c6130ce9d...b32226926e4dce8c68861b737d7c4fb06a6cf276) | Three commits: selector fixes and dependencies/version. UI source diff inspected; matched-baseline/browser execution pending. |
| DOCS-20261104 | [a55fd7de…14acd616](https://github.com/Koenkk/zigbee2mqtt.io/compare/a55fd7deaca449e502fefe705b6443af6a58ed86...14acd6165880130fa3c1608aab74cb5f38b2547a) | 4,123 changed files including generated device pages; images, pairing/configuration, weekly schedule, Tuya CLI and external-converter migration guidance. Full review UNKNOWN. |

### Inspected herdsman changes

- `75e47feee0c7cdd5e84b8f364d574b215d7d094e`: include ZCL frame type when
  matching responses with the same command ID; propagate through adapter waiters.
  Inspected `src/adapter/adapter.ts`, adapter-specific changed call sites and
  `test/adapter/z-stack/adapter.test.ts` added test “Wait for same frameType
  resolves on proper cmd”; Ember touchlink fixture updated as well. **DRIFT-001,
  ADAPTER-005**, plan ZB-03. Impact: compatibility fix; no Rust backport yet.
- `3c381485485e1c8abe79a30075ab3a31d54f8f01`: deCONZ driver consumes timeout
  in milliseconds rather than multiplying by 1000; zero/absent uses 60000.
  **DRIFT-002**, future deCONZ stage; hardware and negative-case tests pending.
- `0b8500042853f0c5c75b86416fa3711c1316fb6e`: regenerate cluster types including
  Tuya `dataReportAlt`, correct `trustCenterAddress` spelling and documentation.
  Generated type diff read; check against runtime schema in CONVERTER-005/CV-03
  before updating distributed metadata. No runtime verification credited.
- Biome/Node types/source-map-js/vite dependency changes and 11.2.1 release:
  package/lock diffs inspected, runtime impact not inferred from commit labels.

### Converter review queue (not a catalog-completeness claim)

Initial refresh ended at `3e2c8af36d8d5b3d4b9077b19ac742e6ecaffbf1`.
Initial delta titles include Moes ZM6LT1 fingerprint/current limit (**DRIFT-003**),
SONOFF MINI-ZBRBS pulse, MiBoxer E3-ZR color restore, MINI-ZB2GS actions, six
HYSYIOT additions, MoveSmart fingerprint, T000R_ZG, `_TYST11_yfmeg60f`, TS011F_5
power-on behavior and Moes AM43 tilt. These are not identification evidence for
the user's unspecified Moes switches.

Final refresh additionally found release 26.126.0. Explicit queued subjects:
07P4; 929004456604; Lidl color-temperature fix; X7633; SPM alarm/fault/reset;
legacy MHO-C401N removal; NEO NAS-AB06B2 battery/siren/fingerprint; Danfoss Ally
heating mode; HZC/Namron detection; ZM25TQ work_state/remote_pairing; Third Reality
3RCB1097Z detection and 3RRE0194Z on/off; HY607W-3A time; Slacky fractional VOC;
WNRL34/24; HS209Z/206Z/207Z; D1Z; Lonsonho intermediate position; CPXC 2X4 TUWH;
Philips mixed-color groups; ADEO color-temperature range; EGLO ring endpoint;
EAYCBM-Z-1P; OURTOP fingerprint; Schneider S520619 settings; do_not_disturb docs;
SONOFF SWV battery drain; HOBEIAN controls/illuminance; ZG-109TDS temperature_probe;
JWCOM fingerprint; ZIGEE-PLUG-16A addition/revert; AVATTO detection; TERNCY sensors;
Heiman radar reset; 929004440102; HS-720ES; G08A-Z20T/N02-Z10T; SP-P05D;
Aqara power_on_behavior read; HeatHUB schedules/settings; JKD-839Z; SA-2; HS300TF;
plus dependency/CI changes. Impact **UNKNOWN until per-definition source review**.

Reproduce the complete immutable queue without treating titles as semantics:

```bash
git -C .upstream/zigbee-herdsman-converters log --oneline c18c32d94c77f683fa768618d1fd32b348fdd7b9..367100282578111770f1c8ab0e38805d9ec581d0
git -C .upstream/zigbee-herdsman-converters diff --stat c18c32d94c77f683fa768618d1fd32b348fdd7b9..367100282578111770f1c8ab0e38805d9ec581d0
```

Disposition: EVID-02/CV-09 must split and trace each actual changed behavior into
per-definition inventory/test cases. The current inventory is incomplete; these
untriaged deltas are not silently counted as supported or as zero open changes.

### Frontend and documentation review queue

Windfront `HeaderDeviceSelector` and `HeaderGroupSelector` now retain the selected
entity in options and set a default value rather than displaying a placeholder
(`56bd192`, `9812035`); `b322269` updates dependencies/version. UI-01..03 must test
selection and source identity with matched package versions. No frontend
compatibility percentage follows from serving assets.

Documentation changed far beyond images. Review generated catalog changes
against converter sources, plus actual installation/configuration/support pages.
The full diff remains pending; a successful fetch does not finish this audit.

### Audit disposition

Twenty narrow discovery rows added; none marked VERIFIED and no passing cases
added. Legacy inventory defects remain explicitly quarantined for EVID-02/03.
All five repository delta groups above remain open at least in part. R1 release
scope is recorded separately and does not exclude future adapters/global work.

## 2026-11-02T15:00Z
- Fetch: zigbee2mqtt 344b6e03 → 3c841f (2.14.2 → 2.14.3), herdsman 3cb89ad (10.10.0) → c1a54e3 (11.2.0) drift, converters 9dabbe4 (26.115.1) → c18c32d (26.125.0) drift, windfront fbe6e2d → 353c7d8 (7 commits)
- Drift impact: herdsman 11.2.0 includes Tuya EF00 fix (#1924) and group DB fix (#1923); converters 26.125.0 adds devices; z2m 2.14.3 bumps deps + availability ping loop fix (#33504), HA fixes. Baseline remains pinned at 2.14.2/10.10.0/26.115.1 per AGENTS baseline advancement policy — not auto-moved. Next baseline advancement requires triage.
- Freshness: DRIFT_DETECTED

## 2026-10-31T00:04Z
- Re-checked zigbee2mqtt 344b6e03, herdsman b560461e, converters c18c32d9 — no drift (HEAD == origin/HEAD for all).
- Added evidence docs/parity/evidence/ADAPTER-znp-init.md tracing zStackAdapter.ts + manager.ts init sequence.
- Fixed parity methodology: Global denominator 38 → 43 after adding stage-2 adapter families (Ember/deCONZ/ZiGate/ZBoss/ZoH). Stage 1 now 14/38, Global 14/43.
- Rust changes: added SYS ping (3-attempt), STARTUP_OPTION + APS_USE_EXT_PANID NV writes, NV ID tests, clippy fixes.

## 2026-10-31T00:00Z
- Baseline set to zigbee2mqtt 344b6e03 (v2.14.2), zigbee-herdsman b560461e (10.10.0), converters c18c32d9 (26.115.1).
- Freshness: CURRENT (remote HEAD matches local).
