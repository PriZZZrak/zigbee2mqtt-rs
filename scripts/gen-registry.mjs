import fs from 'node:fs';
import path from 'node:path';
const dist = process.argv[2] || 'hc/dist';
const ref = process.argv[3] || 'unknown';
const outDir = 'herdsman-converters';
fs.mkdirSync(outDir, {recursive: true});

// Try dist first, fallback to src
let deviceDir = path.join(dist, 'devices');
if (!fs.existsSync(deviceDir)) {
  // Fallback to src/devices in the checked-out repo
  const alt = path.join(path.dirname(dist), 'src', 'devices');
  if (fs.existsSync(alt)) deviceDir = alt;
  else {
    // Try .upstream path
    const up = '/config/workspace/zigbee2mqtt-rs/.upstream/zigbee-herdsman-converters/src/devices';
    if (fs.existsSync(up)) deviceDir = up;
  }
}

const files = fs.readdirSync(deviceDir).filter(f=>f.endsWith('.js') || f.endsWith('.ts'));
const registry = {};
for (const f of files) {
  const content = fs.readFileSync(path.join(deviceDir, f), 'utf8');
  // Extract model and vendor via regex: model: 'L101Ze', vendor: 'Lytko'
  const modelMatches = [...content.matchAll(/model:\s*['"]([^'"]+)['"]/g)];
  const vendorMatches = [...content.matchAll(/vendor:\s*['"]([^'"]+)['"]/g)];
  const base = path.basename(f, path.extname(f));
  // Map each model to file
  for (const m of modelMatches) {
    const model = m[1];
    registry[model] = `${base}.js`;
    // Also map vendor/model combo if vendor found
    if (vendorMatches.length > 0) {
      const vendor = vendorMatches[0][1];
      registry[`${vendor}/${model}`] = `${base}.js`;
      registry[`${vendor.toLowerCase()}/${model}`] = `${base}.js`;
      registry[`${vendor.toLowerCase()}/${model.toLowerCase()}`] = `${base}.js`;
    }
  }
  // Fallback: map base filename
  if (!registry[base]) registry[base] = `${base}.js`;
  // Also map vendor lower
  const vendorFromFile = base.split('_')[0];
  if (!registry[vendorFromFile.toLowerCase()]) registry[vendorFromFile.toLowerCase()] = `${base}.js`;
}
fs.writeFileSync(path.join(outDir, 'registry.json'), JSON.stringify({ref, registry}, null, 2));
console.log(`Generated registry with ${Object.keys(registry).length} entries for ${ref} from ${deviceDir}`);
