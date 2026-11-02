import fs from 'node:fs';
import path from 'node:path';
const dist = process.argv[2] || '/tmp/hc/dist';
const ref = process.argv[3] || 'unknown';
const outDir = 'herdsman-converters';
fs.mkdirSync(outDir, {recursive: true});
const devices = fs.readdirSync(path.join(dist, 'devices')).filter(f=>f.endsWith('.js'));
const registry = {};
for (const f of devices) {
  const base = path.basename(f, '.js');
  // naive vendor/model from filename, real would parse definition
  registry[base] = `${base}.js`;
}
fs.writeFileSync(path.join(outDir, 'registry.json'), JSON.stringify({ref, registry}, null, 2));
console.log(`Generated registry with ${Object.keys(registry).length} entries for ${ref}`);
