# Backlog

## PERF-001 — report delay during manual pairing (low priority)

- **Observed:** one report can wait up to ~0.85 s while `prepareDefinition` of a
  newly paired device occupies the single JS worker. Measured on Routerich:
  300 report samples, 52 overlapping a 120 ms/module Tuya download; p50/p95
  unchanged (2.50/6.02 ms baseline, 2.66/6.10 ms during pairing), max 847 ms.
- **Cause:** single-threaded JS realm; the download itself runs on blocking
  threads and does not stall reports.
- **Decision (2026-11-26):** accepted as low-priority technical debt. Do not add
  a second JS worker or complex `prepareDefinition` isolation for this spike.
  The behaviour matches the upstream single-threaded Node event loop.
- **Revisit only if:** repeated/long stalls are measured outside manual pairing,
  or pairing becomes frequent enough to affect SLA. Then evaluate a scheduler
  priority for prepare jobs, not a second realm.
