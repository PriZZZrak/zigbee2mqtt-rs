# EVID-02 — inventory repair and parity accounting

Date: 2026-11-04. Rust code unchanged at `07bb90f1012e41897ce1d30f79e45a5cfd01ecf2`.
Companion: [AUDIT-20261104](AUDIT-20261104.md), [plans](../../../.opencode/plan/02-parity-and-ci.md).

## Why rows changed

The pre-audit CSV mixed correct labels with three systematic defects. Repair was
mechanical and traceable; no behavior was removed from scope and no verified
progress was earned.

| Defect | Rows | Repair |
|---|---|---|
| `owner` held `UNVERIFIED` instead of an owner | 38 | set to `unassigned`; verification belongs in `status` |
| Windfront/future-adapter rows shifted right by one column (`owner=no`, note text in `upstream_changed`) | 31 | moved the note into `notes`, set `upstream_changed=no`, `owner=unassigned` |
| `baseline_sha` was a 7/8-character prefix or belonged to the wrong repository | 75 | replaced with the full SHA of the source actually referenced: app, herdsman or converters |
| Stale upstream paths | 15 | point converters `fromZigbee.js` → `.ts`; replace the removed `lib/zigbee2mqtt.ts` with `controller.ts` + `eventBus.ts`; database import references point to herdsman `controller/database.ts` |
| `VERIFIED` without `evidence_ref` | 13 | **reopened** to `IMPLEMENTED_UNVERIFIED`, `verified_cases=0`, audit note appended |
| Unvalidated `verified_cases` on non-verified rows (mostly `1/1` stubs) | 38 | reset to `0`, audit note appended; `total_cases` retained as a historical work estimate |

Reopened IDs: `RUNTIME-001`, `CONFIG-001`, `CONFIG-002`, `MQTT-002`,
`ZCL-001`…`ZCL-009`. Their historical `total_cases` remain so the work is not
lost; they contribute zero verified scenarios until a pinned-reference test is
recorded. This is the accounting consequence of AGENTS §6, not a claim that the
corresponding parsers are broken.

## What was not established

- No semantic decomposition of broad rows (`CONVERTER-006`, `WINDFRONT-*`
  wildcards, combined `CONFIG-002`/`STORAGE-001`) — tracked by EVID-02 in the plan.
- No re-count of scenario cases beyond zeroing unproven verified cases.
- No reference/differential execution; Node remains unavailable.
- Classic frontend 0.9.21 git mapping and matched Windfront 2.15.0 reference
  execution remain open.
- No hardware verification.

## Validation now enforced

`scripts/parity_report.py` validates the exact CSV schema, unique IDs, status and
priority enums, positive weights, positive stages, full pinned baseline SHAs,
`0 <= verified_cases <= total_cases`, existing `evidence_ref` files, approval for
`OUT_OF_SCOPE_APPROVED`, and rejects `VERIFIED` without evidence, test refs, all
cases passing, or with unresolved hardware notes. The dashboard region in
`docs/parity/README.md` is generated from the CSV; `--check` fails when it is
stale. `scripts/tests/test_parity_report.py` covers the negative cases.

This gate is necessary but not sufficient for parity: it cannot detect a test
that asserts its own fabricated JSON, nor prove upstream equivalence. Those need
the reference harness (EVID-04).
