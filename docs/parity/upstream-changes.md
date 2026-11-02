# Upstream Changes

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