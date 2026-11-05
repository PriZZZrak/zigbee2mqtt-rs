# Decisions

## 2026-11-05 — R1 hardware measurements and two operational decisions

Measured on the real device (not assumed): Routerich AX3000 v1, RouteRich/OpenWrt
24.10.8, aarch64, **54.3 MB overlay with 5.6 MB free** before the test, 497 MB
RAM; SONOFF Dongle-P on `/dev/ttyUSB0` (cp210x), ZNP 2.7. The pre-existing
`node-zigbee2mqtt` + `/opt/zigbee2mqtt` used 53.1 MB and `node127` another
~29 MB. The user explicitly approved removing the Node version for the test
("просто на время удали нод версию"); rollback is possible because
`node-zigbee2mqtt 2.14.1-r2` remains in the Routerich feed. A verified backup of
`/etc/zigbee2mqtt` was taken first.

1. **Resume on adapter/backup consistency.** Real CC2652P firmware returns a
   116-byte aligned NIB and a raw 16-byte `PRECFGKEY`; a missing/different
   alternate-key descriptor previously made `determine_strategy` select
   `RestoreBackup` and abort. When the backup matches the adapter (PAN, extended
   PAN, channel, active key) and the configuration matches that backup, the
   bridge now **resumes with a warning instead of refusing**. It never rewrites
   NV. Upstream refuses in this situation; the deviation is justified because
   refusing makes a known-good, unchanged network unstartable and is recorded
   here rather than silently accepted.
2. **Converter delivery for R1.** The configured Pages URL
   (`.../herdsman-converters/latest`) serves only `registry.json`; device files
   return 404 and the bridge retried `nous.js` every 5 s. For the HIL test the
   pinned `v26.115.1` package was staged to
   `/etc/zigbee2mqtt-rs/herdsman-converters` (18.3 MB). Publishing the full
   package (or bundling a validated subset) remains an open delivery decision.


## Conversation 2026-11-04 — release scope and priorities

These decisions were explicitly supplied by the user during the planning audit.
They approve release sequencing, not claims of implemented compatibility.

1. **R1 hardware:** SONOFF Dongle-P only. CC2531 is no longer a mandatory hardware
   gate for the first release. Keep CC2531 code/regressions and its full Stage 1
   hardware-verification backlog. No `OUT_OF_SCOPE_APPROVED` rows are authorized.
2. **R1 deployment:** Routerich AX3000 v1, OpenWrt 24.10.8, reported total storage
   54 MB. RAM, writable/free space and exact ABI remain unknown until measured.
3. **R1 priorities:** original → Rust → original migration, Lytko “L101 NS”,
   existing Moes switches and temperature sensor. Zigbee identities/firmware are
   unknown; future radiator thermostats have not been selected.
4. **Generic device processing:** the same upstream-definition mechanism serves
   the whole supported catalog. Personal devices receive development/testing
   priority, with no hardcoded exemptions or model whitelist in Rust.
5. **Converter delivery:** build and publish valid compiled JavaScript entirely
   in Actions, versioned plus latest; persistent cache at
   `/etc/zigbee2mqtt-rs/herdsman-converters`. No manual reconstruction of Pages.
6. **Converter activation:** validate the package before activation, pin the
   working version and support rollback. Automatic/manual activation, retention
   and update cadence have not been decided.
7. **Documentation:** user permits correcting AGENTS/README and discussing
   strategic choices. This audit does not approve runtime architectural changes.

Full Stage 1 still includes CC2531 and Dongle-P plus all shared application
behavior; global scope includes every pinned adapter family. R1 is reported
through a separate acceptance matrix and cannot increase verified parity by
removing unresolved features from denominators.

## Unresolved choices

Matched R1 baseline; generic/native dispatch arbitration; Boa context/bundling
strategy and resource budget; durability of batched writes; pure-Rust TLS;
license/provenance of distributed code/assets. See
[the ADR backlog](../../.opencode/plan/11-decisions-and-questions.md).
