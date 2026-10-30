# Parity Dashboard

Compatibility baseline: zigbee2mqtt 344b6e03 (v2.14.2) + zigbee-herdsman 10.10.0 + converters 26.115.1
Latest upstream check: 2026-10-31 UTC, CURRENT
Stage 1 Z-Stack parity: 14/38 = 36.84% (CC2531 + CC2652P)
Global upstream parity: 14/38 = 36.84%
Stage 1 / global weighted: 26.23% / 26.23%
Stage 1 / global scenarios: 39/128
Status breakdown: VERIFIED 14, IMPLEMENTED_UNVERIFIED 18, PARTIAL 4, DISCOVERED 2
Critical P0 gaps: ADAPTER-001, ADAPTER-002, ADAPTER-003, ZIGBEE-001, ZIGBEE-002, MQTT-001, MQTT-003, MQTT-004, MQTT-006, HA-001
Adapter coverage: Z-Stack (CC2531, CC2652P) implemented but not hardware-verified; other families not started
Inventory audit: preliminary, not complete
Device definition parity: not yet inventoried
Converter parity: not yet inventoried
Open upstream deltas: 0 (baseline current)
Last test run: cargo test (2026-10-31) - pass
Known exclusions: none approved
Notes: Inventory is preliminary. Percentages are based on 38 inventoried behaviors only.