# STORE-03 — shutdown flush and write failure visibility

Rust implementation: `src/bridge.rs` (`shutdown_signal`, `flush_database`,
`persist_database`), `src/database.rs` (`PersistentDatabase::write`). Behavior:
STORAGE-005; plan STORE-03.

## Source contract and original tests

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2` and herdsman
`3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- Herdsman `src/controller/database.ts` writes after mutations; the app's
  controller/extension shutdown path runs `stop()` handlers before exit
  (`lib/controller.ts` lifecycle, tested by `test/controller.test.ts`).
- The Rust bridge batches database flushes by 500 ms. A service stop must not
  drop the last accepted mutation, and write errors must be observable.

The complete original shutdown ordering (MQTT offline publication, database
flush, extension teardown) still needs a line-by-line trace before this row can
leave `IN_PROGRESS`; the current check is source-location level, not a captured
reference sequence.

## Change

1. `shutdown_signal()` awaits SIGTERM (service manager) or Ctrl-C. Previously no
   signal handler existed, so procd's SIGTERM terminated the process without
   running the post-loop offline publication or database flush.
2. `flush_database()` forces the pending write at shutdown and logs failures at
   error level. It clears the debounce flag only after a successful write.
3. The debounced writer now logs failures as errors instead of warnings and
   keeps the in-memory records; the next mutation or shutdown retries.
4. `PersistentDatabase::write` fsyncs the parent directory after the atomic
   rename, making the rename durable where the filesystem supports it. An
   unsupported directory sync is reported but does not fail the write.

## Tests added

- `write_is_atomic_and_replaces_stale_tmp` (`src/database.rs`): a leftover
  `.tmp` from a crashed write is replaced, the temporary file is gone, and the
  record is readable after reopening.

## Known gaps (not verified)

- No integration test sends a real SIGTERM to a running bridge; the signal branch
  has not been exercised against procd or systemd.
- ENOSPC, read-only filesystem and power-loss injection are not simulated.
- The offline MQTT publish happens before the flush; if the broker is down the
  flush still runs, but ordering under failure is not yet covered by a test.
- Debounce can still coalesce two accepted mutations; durability guarantees for
  crash windows (not orderly shutdown) remain an open decision (ADR-05).
