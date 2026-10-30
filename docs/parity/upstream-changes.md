# Upstream Changes

## 2026-10-31T00:04Z
- Re-checked zigbee2mqtt 344b6e03, herdsman b560461e, converters c18c32d9 — no drift (HEAD == origin/HEAD for all).
- Added evidence docs/parity/evidence/ADAPTER-znp-init.md tracing zStackAdapter.ts + manager.ts init sequence.
- Fixed parity methodology: Global denominator 38 → 43 after adding stage-2 adapter families (Ember/deCONZ/ZiGate/ZBoss/ZoH). Stage 1 now 14/38, Global 14/43.
- Rust changes: added SYS ping (3-attempt), STARTUP_OPTION + APS_USE_EXT_PANID NV writes, NV ID tests, clippy fixes.

## 2026-10-31T00:00Z
- Baseline set to zigbee2mqtt 344b6e03 (v2.14.2), zigbee-herdsman b560461e (10.10.0), converters c18c32d9 (26.115.1).
- Freshness: CURRENT (remote HEAD matches local).