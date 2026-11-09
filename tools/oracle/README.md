# Parity oracle

Generates and replays differential fixtures against the pinned
`zigbee-herdsman` checkout in `.upstream/`.

## Toolchain (already installed on this machine)

- Node: `/config/.local/node-v24.22.2-linux-x64`
- Symlinks: `/config/.local/bin/{node,npm,npx,tsx}`
- `tsx`: `4.23.15` (global, same npm prefix)
- Upstream pin: `zigbee-herdsman` v11.2.0 (`c1a54e3d`)

Check:

```bash
node --version   # v24.22.2
tsx --version    # tsx v4.23.15
```

Do not download Node into `/tmp` per session. If the symlinks are missing,
recreate them against `/config/.local/node-v24.22.2-linux-x64/bin/`.

## Generate committed fixtures

```bash
tsx tools/oracle/generate.mjs
```

Writes:

```
crates/zigbee-apiarist/tests/fixtures/frame.json
crates/zigbee-apiarist/tests/fixtures/buffalo.json
crates/zigbee-apiarist/tests/fixtures/buffalo_znp.json
crates/zigbee-apiarist/tests/fixtures/zpi_object.json
crates/zigbee-apiarist/tests/fixtures/zcl.json
crates/zigbee-apiarist/tests/fixtures/constants.json
crates/zigbee-apiarist/tests/fixtures/definition.json
crates/zigbee-apiarist/tests/fixtures/structs.json
```

The ZNP command catalog and constants modules are generated directly into
`src/` by `gen-definition.mjs` / `gen-constants-rust.mjs` (also invoked by
`npm run generate`).

## Run the live oracle

Reads NDJSON jobs from stdin and prints NDJSON results:

```bash
printf '%s\n' \
  '{"id":"a","module":"frame","op":"to_buffer","type":"SRSP","subsystem":"SYS","commandId":3,"data":"0601"}' \
  | tsx tools/oracle/run.mjs
```

## Add a module

1. Add `packages/<module>.mjs` exporting `handle(job)`.
2. Register it in `run.mjs` and `generate.mjs`.
3. Add cases to `generate.mjs` and regenerate.
4. Add `crates/zigbee-apiarist/tests/differential_<module>.rs`.
5. Update `docs/parity-matrix.md`.

The Rust gate (`cargo test`) only reads the committed JSON; it never requires
Node. Node is needed only to regenerate fixtures or run the live oracle.
