# Fuzz/property corpus (CP-10)

Deterministic, bounded seeds only; no external fuzz crate and no hardware.

## Corpus

- `tools/oracle/fuzz-corpus.mjs` generates **120 UNPI frame cases** from the
  fixed seed `0x5eed1234` (LCG) against the pinned Node oracle
  (`zigbee-herdsman` v11.2.0, `c1a54e3d`). Half get a valid checksum, half stay
  malformed; buffers cover truncated FCS positions, empty data and random command
  bytes. The result is committed as
  `crates/zigbee-apiarist/tests/fixtures/fuzz_unpi.json` (53 accepted / 67
  rejected by the upstream parser).
- Rust replay: `crates/zigbee-apiarist/tests/fuzz_boundaries.rs`
  (`unpi_malformed_corpus_matches_upstream`) must match the oracle's
  accept/reject decision and the parsed type/subsystem/command/data. The parser
  now mirrors Node's `subarray(4, fcsPosition)` behaviour for overlapping FCS
  instead of panicking.
- Property checks in the same file use the fixed seed `0x5eed1234` /
  `0xfeed9876`: 500 Buffalo write→read round-trips, 500 truncated reads (must
  return or raise the Node-style range error), and 500 random ZCL buffers
  (rejected or re-serialised and re-parsed deterministically).

## Commands

```bash
node tools/oracle/fuzz-corpus.mjs            # live Node oracle, rewrites fixture
cargo test -p zigbee-apiarist --test fuzz_boundaries
```

Live-oracle refresh is a separate run from the committed offline replay so the
normal Rust gate does not need Node.

## Contract exclusions

- `Type` and `Subsystem` are Rust enums. UNPI frames with a 5-bit subsystem
  outside the enum are out of contract for the typed parser; the corpus keeps
  cmd0 valid and randomises the rest. Upstream's `Frame` accepts any number and
  only fails later in `ZpiObject`.
- Random ZCL specific-command payloads whose parameter shape cannot be encoded
  are treated as rejected inputs; the property test only re-serialises global
  frames.
- Round-trip Rust→Rust is a robustness check, not parity evidence; parity comes
  from the Node-generated UNPI corpus and the differential fixtures
  (`differential_buffalo_znp`, `differential_frame`, `differential_zcl`).

## Results (2026-11-18)

```text
node tools/oracle/fuzz-corpus.mjs
wrote 120 UNPI fuzz cases (53 valid)

cargo test -p zigbee-apiarist --test fuzz_boundaries
test result: ok. 3 passed; 0 failed
```
