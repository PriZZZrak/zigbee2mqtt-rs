#!/usr/bin/env node
// Write an integrity manifest for a staged converter package.
// Usage: node scripts/gen-manifest.mjs <package-dir> <ref>
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const [root, ref] = process.argv.slice(2);
if (!root || !ref) {
    console.error("usage: gen-manifest.mjs <package-dir> <ref>");
    process.exit(2);
}
if (!fs.existsSync(root) || !fs.statSync(root).isDirectory()) {
    console.error(`package directory does not exist: ${root}`);
    process.exit(2);
}

function walk(dir, prefix = "") {
    const out = [];
    for (const entry of fs.readdirSync(dir, {withFileTypes: true})) {
        const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) {
            out.push(...walk(path.join(dir, entry.name), relative));
        } else if (entry.isFile() && relative !== "manifest.json") {
            out.push(relative);
        }
    }
    return out.sort();
}

const files = walk(root).map((relative) => {
    const bytes = fs.readFileSync(path.join(root, relative));
    return {
        path: relative,
        bytes: bytes.length,
        sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
    };
});
const counts = {devices: 0, converters: 0, lib: 0, other: 0};
for (const file of files) {
    const top = file.path.split("/")[0];
    if (top in counts) counts[top] += 1;
    else counts.other += 1;
}
const manifest = {
    schema: 1,
    ref,
    generated_at: new Date().toISOString(),
    counts,
    files,
};
fs.writeFileSync(path.join(root, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`manifest ${ref}: ${files.length} files, ${counts.devices} devices`);
