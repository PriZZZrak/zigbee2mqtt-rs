# Decisions

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
