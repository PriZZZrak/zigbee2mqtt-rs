# Parity Dashboard

Compatibility baseline: zigbee2mqtt 344b6e03 (v2.14.2) + zigbee-herdsman 10.10.0 (b560461e) + converters 26.115.1 (c18c32d9)
Latest upstream check: 2026-10-31T00:04Z UTC, CURRENT — all heads match baseline
Freshness: CURRENT (within 24h window, no unreviewed deltas)
Discovered: 43 eligible behaviors; 5 with 0 cases (future adapters)
Stage 1 Z-Stack parity: 14/38 = 36.84% [CC2531 + CC2652P mandatory] — preliminary (inventory incomplete)
Global upstream parity: 14/43 = 32.56% [all adapter families] — preliminary, insufficiently defined
Stage 1 / global weighted parity: 26.23% / 21.92%
Stage 1 / global scenarios: 48/128 and 48/128 [5 unmeasured rows]
Status breakdown: VERIFIED 14 / PARTIAL 7 / IMPLEMENTED_UNVERIFIED 15 / DISCOVERED 7 (43 total)
Critical P0 gaps: ADAPTER-001, ADAPTER-002, ADAPTER-003, ZIGBEE-001, ZIGBEE-002, MQTT-001, MQTT-003, MQTT-004, MQTT-006, HA-001
Adapter coverage: Z-Stack CC2531 PARTIAL (3/5), Z-Stack CC2652P PARTIAL (3/5); transport PARTIAL (3/4); Ember/deCONZ/ZiGate/ZBoss/ZoH DISCOVERED (future stage)
Inventory audit: 6/16 domains audited; device catalog and full converter inventory not yet enumerated
Device definition parity: 0/baseline (not inventoried)
Converter parity: 0/baseline (not inventoried)
Open upstream deltas: 0 (baseline current, 2026-10-31 check)
Last test run: cargo test 2026-10-31 — 30 compat + ~107 lib tests pass; cargo check pass; cargo clippy 0 warnings
Known exclusions: none approved — future adapter families deferred, not excluded
Evidence: docs/parity/evidence/ADAPTER-znp-init.md covers Z-Stack init parity (ping + NV fixes)
Notes: Global denominator now correctly exceeds Stage 1 (38 → 43) after adding stage-2 adapter rows. Percentages remain preliminary; full audit required before claiming completeness. Hardware-in-loop verification pending (simulated/compiled only).