# Extension test parity and differential oracle

Upstream oracle: `.upstream/zigbee2mqtt` at `3c841fdf` (zigbee2mqtt 2.14.3),
`test/extensions/*`. Command:

```bash
cd .upstream/zigbee2mqtt
node node_modules/vitest/vitest.mjs run --config ./test/vitest.config.mts \
  --testTimeout=60000 --reporter=json --outputFile=/tmp/upstream_ext.json \
  test/extensions
```

Result on 2026-11-14: **598 / 598 passed** (15 files, 0 failed).

> The default 5s timeout fails the HA duplicate-mapping scan on this machine
> (7s runtime); with the upstream-intended longer timeout it passes 115/115.
> This is an oracle-environment detail, not a product failure.

| Upstream file | Oracle tests | Rust behavioral tests | Notes |
|---|---:|---|---|
| `availability.test.ts` | 31 | `tests/availability.rs` (3) | Timers/backoff/ping are exercised through real Tokio timers, not fake timers; assertions cover publish payloads, dedupe, rename cleanup |
| `bind.test.ts` | 31 | `tests/bind.rs` (3) | Validation/error responses and nothing-to-bind; adapter-backed bind/reporting paths are covered indirectly by `controller_suite` |
| `bridge.test.ts` | 110 | `tests/bridge.rs` (5) | Retained topics, options/health, group add + rename, force remove, custom clusters; touchlink/action/generate-external-definition cases are the approved OMITTED handlers |
| `configure.test.ts` | 19 | `tests/configure.rs` (3) | MQTT request success/unknown/non-configurable; attempts/version paths covered by unit paths in the extension |
| `frontend.test.ts` | 16 | `tests/frontend.rs` (6) | Auth, retained/state replay, inbound dispatch, broadcast filtering, offline on stop, TLS (wss) and unix socket |
| `groups.test.ts` | 40 | `tests/groups.rs` (4) | Optimistic fan-out, dedupe, invalid payload, unknown group/device |
| `health.test.ts` | 5 | `tests/health.rs` (3) | Payload shape, reset_on_check, clear_stats; `mqtt.queued` now tracks in-flight publishes |
| `homeassistant.test.ts` | 115 | `tests/homeassistant.rs` (5) | Discovery for all expose types, bridge entities, triggers, wildcard event types, adjust hook; group aggregation + per-object overrides implemented |
| `networkMap.test.ts` | 6 | `tests/network_map.rs` (4) | graphviz/plantuml/raw and unsupported type; scan behaviour is adapter-backed |
| `onEvent.test.ts` | 3 | `tests/on_event.rs` (1) | start/joined/stop dispatch |
| `publish.test.ts` | 110 | `tests/publish.rs` (4) | Topic parsing, unsupported device, set/get dispatch, optimistic publish |
| `receive.test.ts` | 34 | `tests/receive.rs` (5) | Definition-less fallback, converter payload, unsupported cluster, debounce, conflict; calibration has `tests/calibration.rs` (5) |
| `external*.test.ts` | 27 | — | OMITTED: user JavaScript evaluation is forbidden (AGENTS §5 Q5) |
| `otaUpdate.test.ts` | 51 | — | OMITTED: OTA is out of R1 (AGENTS §4) |

## Differential status

- The upstream suites above run green and are the behavioural oracle for the
  ported surface.
- Rust counts are intentionally behavioural slices rather than a 1:1 port of
  upstream mock-heavy unit tests; every ported file has integration tests that
  assert observable MQTT payloads, discovery topics, responses and logs.
- Differences tracked in `docs/parity-matrix.md`: approved bridge OMITTED
  handlers, deferred Windfront static assets, and `bridge/info.os.node_version`
  (no Node runtime; `"rust"` placeholder value is documented in the row).
- Remaining per-case work (if strict 1:1 test-count parity is required):
  availability fake-timer matrix, bind adapter-backed reporting, bridge
  diagnostics/touchlink cases (OMITTED), HA per-device icon/override matrix.
