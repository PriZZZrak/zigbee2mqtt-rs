#!/usr/bin/env node
// Consistency checks for the parity/test maps in docs/.
//
// Checks:
//   1. every `docs/*-parity.md` that names an upstream test file also states its
//      `it` case count, and that count matches the file on disk;
//   2. every backticked snake_case identifier in those maps exists as a Rust
//      test function (or is a known non-test symbol);
//   3. every `docs/...`, `tests/...`, `crates/...` path mentioned in the maps
//      exists;
//   4. `docs/extension-test-cases.md` per-class counts sum to its declared total;
//   5. the docs referenced from `docs/parity-matrix.md` exist.
//
// Usage: node tools/docs/check-parity-maps.mjs
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const errors = [];
const warnings = [];

function walk(dir, filter, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target" || entry.name === "node_modules" || entry.name === ".git") continue;
      walk(path, filter, out);
    } else if (filter(entry.name)) {
      out.push(path);
    }
  }
  return out;
}

const rustFiles = walk(join(root, "crates"), (name) => name.endsWith(".rs"));
const rustSources = rustFiles.map((file) => ({ file, text: readFileSync(file, "utf8") }));
// Docs may reference a module or a test suite by its file stem.
const moduleStems = new Set(
  rustFiles.map((file) => file.split("/").pop().replace(/\.rs$/, "")),
);
function rustSymbolExists(name) {
  if (moduleStems.has(name)) return true;
  const needle = new RegExp(`\\b(?:fn|type|struct|enum|const)\\s+${name}\\b`);
  if (rustSources.some(({ text }) => needle.test(text))) return true;
  // Struct fields are referenced by name in the maps too.
  const field = new RegExp(`\\bpub ${name}\\s*:`);
  return rustSources.some(({ text }) => field.test(text));
}

const docsDir = join(root, "docs");
const maps = readdirSync(docsDir)
  .filter((name) => name.endsWith("-parity.md") || name === "extension-test-cases.md")
  .map((name) => join(docsDir, name));

// 1 + 2 + 3: upstream counts, test names, paths.
for (const doc of maps) {
  const text = readFileSync(doc, "utf8");
  const rel = relative(root, doc);

  const source = text.match(/\.upstream\/[^\s`)]+test\.ts/);
  const claimed = text.match(/(\d+)\s*(?:`it`|\bit\b)\s*cases?/);
  if (source) {
    const upstream = join(root, source[0]);
    if (!existsSync(upstream)) {
      errors.push(`${rel}: upstream source ${source[0]} does not exist`);
    } else if (claimed) {
      const actual = (readFileSync(upstream, "utf8").match(/\bit\(/g) || []).length;
      const declared = Number(claimed[1]);
      if (actual !== declared) {
        errors.push(`${rel}: claims ${declared} it cases, ${source[0]} has ${actual}`);
      }
    }
  } else if (!rel.endsWith("extension-test-cases.md")) {
    warnings.push(`${rel}: no upstream source reference`);
  }

  // Backticked identifiers that look like Rust test/helper names.
  const names = new Set(
    [...text.matchAll(/`([a-z][a-z0-9_]{6,})`/g)].map((match) => match[1]),
  );
  const allow = new Set([
    "snake_case",
    "tcp_discovery",
    "accept_joining",
    "device_options",
    "messageemitted",
    "cache_state",
    "log_level",
    "cache_state_persistent",
    "state_change_reason",
    "action_duration",
    "force_disable_retain",
    "disable_default_response",
  ]);
  for (const name of names) {
    if (allow.has(name)) continue;
    if (name.includes("_") && !rustSymbolExists(name)) {
      errors.push(`${rel}: \`${name}\` is not a Rust symbol`);
    }
  }

  for (const match of text.matchAll(/`((?:docs|tests|crates)\/[A-Za-z0-9_./-]+)`/g)) {
    const path = join(root, match[1]);
    if (!existsSync(path)) errors.push(`${rel}: path ${match[1]} does not exist`);
  }
}

// 4: extension-test-cases per-file rows must sum to their case count.
{
  const file = join(docsDir, "extension-test-cases.md");
  const text = readFileSync(file, "utf8");
  const rows = [...text.matchAll(
    /^\|\s*([a-zA-Z]+\.test\.ts)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|/gm,
  )];
  if (rows.length === 0) {
    errors.push("extension-test-cases.md: summary rows not parsed");
  } else {
    let sum = 0;
    for (const row of rows) {
      const cases = Number(row[2]);
      const classes = row
        .slice(3, 10)
        .map(Number)
        .reduce((left, right) => left + right, 0);
      sum += cases;
      if (cases !== classes) {
        errors.push(
          `extension-test-cases.md: ${row[1]} declares ${cases} cases but classes sum to ${classes}`,
        );
      }
    }
    console.log(`extension-test-cases.md: ${rows.length} files, ${sum} cases`);
  }
}

// 5: parity-matrix doc references.
{
  const text = readFileSync(join(docsDir, "parity-matrix.md"), "utf8");
  for (const match of text.matchAll(/`(docs\/[A-Za-z0-9_./-]+\.md)`/g)) {
    if (!existsSync(join(root, match[1]))) {
      errors.push(`parity-matrix.md: missing ${match[1]}`);
    }
  }
}

for (const warning of warnings) console.log(`WARN ${warning}`);
for (const error of errors) console.log(`ERROR ${error}`);
console.log(
  `${errors.length === 0 ? "OK" : "FAILED"}: ${maps.length} maps checked, ${errors.length} errors, ${warnings.length} warnings`,
);
process.exit(errors.length === 0 ? 0 : 1);
