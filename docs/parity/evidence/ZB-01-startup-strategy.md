# ZB-01 — startup strategy (resume / restore / commissioning)

Rust implementation: `src/coordinator/znp/strategy.rs` (decision + parsers),
`src/coordinator/znp/mod.rs` (`determine_startup`, NV probe, startup branch),
`src/coordinator/mod.rs` (config path passed to the coordinator).
Behavior: ADAPTER-004; plan ZB-01.

## Source contract

Pinned herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- `adapter/z-stack/adapter/manager.ts:55–104` (`start`): reads NV, determines a
  strategy, then runs `beginStartup`, `beginRestore` or `beginCommissioning`;
  Z-Stack 1.2 recommissions instead of restoring.
- `manager.ts:110–279` (`determineStrategy`): reads `hasConfigured`, NIB,
  preconfigured/active/alternate keys and the stored backup; computes
  `configMatchesAdapter`, `backupMatchesAdapter`, `configMatchesBackup`; applies
  the reversed/default extended-PAN exceptions and the Z-Stack 1.2 backup-version
  guard; error on adapter/backup/config inconsistency unless forced.
- `manager.ts:516–545` (`parseConfigNetworkOptions`): replaces the default
  `dd:dd:…` extended PAN with the coordinator IEEE address and records
  `hasDefaultExtendedPanId`.
- `adapter-nv-memory.ts:26–30,56–88`: NV reads use `osalNvLength` +
  chunked `osalNvReadExt`; alignment is detected via `NWKKEY` length.
- Original tests: `test/adapter/z-stack/adapter.test.ts`, manager/backup tests.

## Rust change

- Added the pure decision state machine with the source branch order:
  unconfigured/no-NIB → backup/config match → restore; configured and matching →
  startup; mismatch + backup matching adapter → error (force overrides);
  mismatch + backup matching config → restore; otherwise commissioning.
- Added parsing for the unaligned 112-byte NIB, `hasConfigured` (`0x55`) and
  17-byte key descriptors, plus unified (`zigpy/open-coordinator-backup`)
  backup loading. Unknown/legacy backup formats are rejected instead of
  commissioning over them.
- ZNP startup now probes NV and:
  - **Startup:** resumes without rewriting NV or BDB channel (the previous code
    rewrote NV on every start).
  - **RestoreBackup:** returns an explicit error; backup restore is not
    implemented and must not silently re-commission.
  - **StartCommissioning:** keeps the existing NV + BDB commissioning path.
  - **Probe unsupported (NV length/read unavailable or non-112 NIB):** logs a
    warning and uses the legacy NV-write path, explicitly marked transitional.
- Config path is passed into coordinator startup so `coordinator_backup.json` is
  read from the application data directory.

## Tests

`cargo test --locked` → **291 passed** (259 library, 2 binary, 30 compat).
`coordinator::znp::strategy` contains 18 tests:

- matching configured adapter resumes; Z-Stack 3 preconfigured-key mismatch only
  warns; Z-Stack 1.2 requires it;
- reversed extended PAN accepted with warning; default `dd` extended PAN
  accepted;
- unconfigured without backup commissions; with matching backup restores;
- Z-Stack 1.2 rejects a Z-Stack 3 backup;
- inconsistent adapter+backup errors by default and starts only with force;
- config-matching backup restores; backup matching nothing commissions;
- truncated/misaligned input rejection;
- NIB/`hasConfigured`/key descriptor parsers and unified backup loading, including
  rejection of an unknown backup format.

## Divergences and gaps (not verified)

- No hardware-in-loop run; all NV reads/startup branches are unit-level.
- Z-Stack 1.2 upstream reads the preconfigured key through SAPI
  `readConfiguration`; Rust reads NV `PRECFGKEY` instead. A real CC2531 fixture is
  required before claiming this branch.
- Aligned-memory NIB layouts (NWKKEY length 24) are not parsed; the explicit
  legacy fallback is used, which rewrites NV. This is documented, not parity.
- `RestoreBackup` is refused; implementing restore is destructive and still needs
  a controlled hardware procedure.
- Backup loading supports only the unified format; legacy raw backups are
  rejected.
- No test drives `determine_startup` against a fake ZNP stream yet; the pure
  decision and parsers are tested, the transport probe is not.
