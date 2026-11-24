# Converter artifact generator

Build-time Node tool for the fingerprint-aware converter artifact.

```bash
node tools/artifact-generator/generate.js \
  --converters .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters \
  --babel-parser /path/to/@babel/parser \
  --behavior-probe target/debug/runtime_probe \
  --out target/converter-artifact

node tools/artifact-generator/resolver-oracle.js \
  --converters .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters \
  --index target/converter-artifact/index.json \
  --out /tmp/resolver-oracle.json

cargo run -p converter-artifact --bin resolver_diff -- \
  --index-json target/converter-artifact/index.json \
  --index-bin target/converter-artifact/index.z2idx1 \
  --oracle /tmp/resolver-oracle.json
```

Install the build-time parser with `npm install` in this directory (or pass
`--babel-parser /path/to/@babel/parser`). `@babel/parser` is a build-time
dependency only; it is never part of the
runtime or the OpenWrt package. `--behavior-probe` uses the real converter
runtime binary for the tuya fromZigbee/set/configure/onEvent differential. A
split is published only when the behavioral gate is non-skipped and passes;
otherwise the generator falls back to the original module. The explicit
development-only `--allow-unverified-split 1` override exists for research and
must not be used for a production artifact.

The generator writes `index.z2idx1`, `index.json`, `graph.json`, `manifest.json`
and a staged `dist/` tree. Device modules are artifact entries, not package
content. Only `tuya.js` is split by default; `--split-files` can add more files
only when the same equivalence gates pass.

The manifest hash is an integrity checksum of the listed artifacts, not a
cryptographic signature.
