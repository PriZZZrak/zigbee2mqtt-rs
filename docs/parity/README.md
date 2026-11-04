# Parity dashboard — accounting validation blocked

**Accepted Stage 1/global completion percentages: UNKNOWN.** Historical CSV
labels are not evidence: 13 `VERIFIED` rows have no evidence_ref, 31 rows have
invalid shifted metadata, and unsupported scenario counters include stub paths.
The previous 14/43 and CURRENT dashboard is withdrawn.

## Sources and freshness

- Baseline: app `344b6e03b43644c97154a72ca9eae17193c553a2` (2.14.2), herdsman
  `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2` (10.10.0), converters
  `9dabbe4e343573c5a3070605efd843a1b3a16058` (26.115.1).
- Frontend provenance **blocked**: app resolves Windfront 2.15.0; historical lock
  pins 2.15.1. Classic frontend 0.9.21 source mapping is still unknown.
- Final successful fetch: **2026-11-04T09:08:33.742736+00:00**.
  Freshness: **DRIFT_DETECTED**; five repository delta groups remain partly open.
- Latest heads: app `3c841fdf`, herdsman `eb80ba1f`, converters `36710028`,
  Windfront `b3222692`, documentation `14acd616`. Full SHAs and package integrities
  are in [upstream-lock.json](upstream-lock.json); review queue in
  [upstream-changes.md](upstream-changes.md).

## Mechanical diagnostics — NOT accepted parity

Calculated from CSV rows using the AGENTS formulas; no percentages are estimates
of working functionality. “Before” is the 75-row input at audited Rust HEAD;
“after” includes 20 zero-credit discovery rows. Existing labels/counters have
been preserved for traceable repair, not approved by this audit.

| Diagnostic | Before | After |
|---|---|---|
| Stage 1 legacy VERIFIED labels | 13/70 = 18.57% | 13/88 = 14.77% |
| Global legacy VERIFIED labels | 13/75 = 17.33% | 13/95 = 13.68% |
| Stage 1 weighted legacy labels | 14/97 = 14.43% | 14/115 = 12.17% |
| Global weighted legacy labels | 14/109 = 12.84% | 14/129 = 10.85% |
| Stage 1 legacy scenario counters | 87/165 = 52.73% | 87/165 = 52.73% |
| Global legacy scenario counters | 87/165 = 52.73% | 87/165 = 52.73% |
| Unmeasured Stage 1/global rows | 2 / 7 | 20 / 27 |

Current labels: VERIFIED 13, IMPLEMENTED_UNVERIFIED 45, PARTIAL 10, DISCOVERED 27.
**No verified progress earned in this planning change.** Denominators grew through
discovery; feature support did not regress. Broad/overlapping legacy rows also
need decomposition before denominators can represent independently tested
behavior. No approved global exclusions.

Recompute arithmetic with `python3 scripts/parity_report.py`; its output is an
unvalidated legacy diagnostic, not a publishable parity claim. It lacks the
required schema/evidence/freshness validator and Markdown generator. Repair is
specified in [EVID-02/03](../../.opencode/plan/02-parity-and-ci.md). This interim
snapshot explicitly blocks accepted KPIs rather than hiding those failures.

## Coverage and blockers

- Inventory audit: incomplete; no substantiated audited-domains/16 fraction.
  Catalog totals and per-definition/converter parity denominators are UNKNOWN.
- Adapters: zstack, ember, deconz, zigate, zboss, zoh, deprecated ezsp inventoried.
  Z-Stack has implementation/tests; no hardware acceptance was performed here.
  Other adapter implementations/HIL remain future-stage gaps. CC2531 regression
  support is preserved; full Stage 1 requires both CC2531 and Dongle-P HIL.
- R1 is a separate Dongle-P/OpenWrt milestone with
  [36 planned acceptance cases](../../.opencode/plan/10-acceptance-matrix.md),
  not 36 passed scenarios or complete Stage 1.
- P0: CONFIG-003/004, STORAGE-002/003, ADAPTER-004/005, CONVERTER-007..012,
  MQTT-010, RUNTIME-003, existing MQTT/state/HA gaps. Frontend JSON API and real
  request side effects remain release blockers for promised UI operations.
- New drift examples: DRIFT-001 frame-type matching, DRIFT-002 deCONZ timeout,
  DRIFT-003 Moes variant; large catalog/docs delta queues remain unreviewed.

## Last checks

At audited Rust HEAD `07bb90f1012e41897ce1d30f79e45a5cfd01ecf2`, Linux,
2026-11-03 UTC: `cargo fmt --check`, `cargo test -- --list`, `cargo test` pass
(217 library + 30 compatibility = **247**). `cargo clippy --all-targets` exits 0
with warnings. `cargo build --release --locked` passes. ARM GNU build fails on
missing target std/core. Node reference, broker integration, browser E2E,
OpenWrt and hardware tests were not run.

See [audit evidence](evidence/AUDIT-20261104.md) for exact commands/timestamps,
original sources/tests, observed gaps and tool limitations, and
[the completion roadmap](../../.opencode/plan/README.md) for next steps.
