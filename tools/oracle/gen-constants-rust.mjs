// Generates the Rust constant modules from the pinned upstream constants.
//
// Usage: tsx tools/oracle/gen-constants-rust.mjs

import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import AF from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/af.ts";
import DBG from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/dbg.ts";
import MAC from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/mac.ts";
import SAPI from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/sapi.ts";
import SYS from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/sys.ts";
import UTIL from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/util.ts";
import ZDO from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/zdo.ts";

const OUT = fileURLToPath(
    new URL("../../crates/zigbee-apiarist/src/adapter/z_stack/constants/", import.meta.url),
);

const snake = (value) => value.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();

function emitObject(value, indent, lines) {
    const pad = " ".repeat(indent);
    for (const [key, child] of Object.entries(value)) {
        if (typeof child === "number") {
            lines.push(`${pad}pub const ${key}: u32 = ${child};`);
        } else if (child && typeof child === "object") {
            lines.push(`${pad}pub mod ${snake(key)} {`);
            emitObject(child, indent + 4, lines);
            lines.push(`${pad}}`);
        } else {
            throw new Error(`unsupported constants value for '${key}': ${typeof child}`);
        }
    }
}

function flatten(value, prefix, out) {
    for (const [key, child] of Object.entries(value)) {
        if (typeof child === "number") {
            out.push([`${prefix}::${key}`, child]);
        } else if (child && typeof child === "object") {
            flatten(child, `${prefix}::${snake(key)}`, out);
        }
    }
}

function emitModule(file, exportName, object) {
    const lines = [
        `// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/${file}.ts`,
        "// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).",
        "",
        `/// Upstream \`${exportName}\`.`,
    ];
    emitObject(object, 0, lines);
    writeFileSync(`${OUT}/${file}.rs`, `${lines.join("\n")}\n`);
    process.stdout.write(`${file}.rs\n`);
}

const unwrap = (module) => module?.default ?? module;
const modules = [
    ["af", "AF", unwrap(AF)],
    ["dbg", "DBG", unwrap(DBG)],
    ["mac", "MAC", unwrap(MAC)],
    ["sapi", "SAPI", unwrap(SAPI)],
    ["sys", "SYS", unwrap(SYS)],
    ["util", "UTIL", unwrap(UTIL)],
    ["zdo", "ZDO", unwrap(ZDO)],
];

for (const [file, exportName, object] of modules) {
    emitModule(file, exportName, object);
}

const flattened = [];
for (const [file, , object] of modules) {
    flatten(object, `constants::${file}`, flattened);
}

const test = [
    "// Generated from zigbee-herdsman v11.2.0 by tools/oracle/gen-constants-rust.mjs.",
    "// Do not edit by hand; regenerate when the upstream constants change.",
    "",
    "use zigbee_apiarist::adapter::z_stack::constants;",
    "",
    "#[test]",
    "fn generated_constants_match_upstream() {",
    ...flattened.map(([path, value]) => `    assert_eq!(${path}, ${value});`),
    "}",
    "",
];
writeFileSync(
    fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/generated_constants.rs", import.meta.url)),
    test.join("\n"),
);
process.stdout.write("generated_constants.rs\n");

/*
emitModule("af", "AF", unwrap(AF));
emitModule("dbg", "DBG", unwrap(DBG));
emitModule("mac", "MAC", unwrap(MAC));
emitModule("sapi", "SAPI", unwrap(SAPI));
emitModule("sys", "SYS", unwrap(SYS));
emitModule("util", "UTIL", unwrap(UTIL));
emitModule("zdo", "ZDO", unwrap(ZDO));
*/
