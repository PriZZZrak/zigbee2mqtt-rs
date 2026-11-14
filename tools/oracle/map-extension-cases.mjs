#!/usr/bin/env node
// Per-case mapping of the upstream extension suites
// (`.upstream/zigbee2mqtt/test/extensions/*`) onto the Rust port.
//
// Input: the vitest JSON report produced by
//   node node_modules/vitest/vitest.mjs run --config ./test/vitest.config.mts \
//     --testTimeout=60000 --reporter=json --outputFile=/tmp/opencode/upstream_ext.json \
//     test/extensions
//
// Output: `docs/extension-test-cases.md` (one line per upstream case).
//
// Coverage classes:
//   asserted     - a named Rust test asserts the same observable behaviour
//   slice        - a broader Rust test covers part of the behaviour
//   code         - ported and reachable, no dedicated Rust assertion yet
//   converter-db - depends on zigbee-herdsman-converters definitions (R1 Q4)
//   adapter      - needs a live coordinator/adapter (HIL pending)
//   static       - frontend static assets deferred to OpenWrt packaging
//   omitted      - approved OMITTED (AGENTS §5 Q5)
//
// Usage:
//   node tools/oracle/map-extension-cases.mjs /tmp/opencode/upstream_ext.json \
//     docs/extension-test-cases.md

import fs from "node:fs";

const reportPath = process.argv[2];
const outPath = process.argv[3] ?? "docs/extension-test-cases.md";
if (!reportPath) {
    console.error("usage: map-extension-cases.mjs <vitest-json> [out.md]");
    process.exit(1);
}

const A = "asserted";
const S = "slice";
const C = "code";
const CDB = "converter-db";
const AD = "adapter";
const ST = "static";
const OM = "omitted";

const range = (from, to) => Array.from({ length: to - from + 1 }, (_, i) => from + i);
const entries = (indices, entry) => indices.map((i) => [i, entry]);

const CASES = {
    "availability.test.ts": {
        rust: "crates/z2m-rs/tests/availability.rs",
        default: [C, "timer/ping path ported, no dedicated assertion"],
        map: {
            1: [A, "publishes_offline_then_online_after_last_seen"],
            2: [AD, "startup ping needs a live coordinator"],
            3: [AD, "startup ping needs a live coordinator"],
            4: [A, "passive_device_publishes_offline_after_timeout_and_online_on_last_seen (paused clock)"],
            5: [A, "availability_disabled_via_device_option"],
            6: [S, "passive timeout path; the 25h default differs only by the timeout value"],
            7: [AD, "ping timer reset needs a live device"],
            8: [AD, "ping retry needs a live device"],
            9: [AD, "ping timer reset needs a live device"],
            10: [A, "publishes_offline_then_online_after_last_seen"],
            11: [A, "device_timeout_option_controls_the_timer"],
            12: [AD, "ping suppression needs a live device"],
            13: [C, "get_timeout reads availability.{active,passive}.timeout"],
            14: [AD, "ping queue removal on leave"],
            15: [AD, "ping queue removal on remove"],
            16: [C, "start() returns early when availability is disabled"],
            17: [C, "is_availability_enabled_for_entity per-device override"],
            18: [AD, "retrieveState pings through the adapter"],
            19: [A, "rename_publishes_empty_retained_availability"],
            20: [A, "passive_device_publishes_offline_after_timeout_and_online_on_last_seen"],
            21: [C, "group availability fan-out in publish_availability"],
            22: [C, "ping queue cleared on stop"],
            23: [C, "single-run ping queue drain"],
            24: [C, "instance restart guard"],
            25: [C, "jitter via utils::random_f64 in reset_timer"],
            26: [C, "backoff reset on ping success"],
            27: [C, "backoff growth on failures"],
            28: [C, "backoff cleared on last seen"],
            29: [C, "pause_on_backoff_gt handling"],
            30: [C, "MAX_TIMEOUT clamp"],
            31: [C, "backoff disabled via device option"],
        },
    },
    "bind.test.ts": {
        rust: "crates/z2m-rs/tests/bind.rs",
        default: [AD, "bind/reporting goes through a live coordinator"],
        map: {
            2: [A, "invalid_payload_errors"],
            7: [A, "bind_without_supported_clusters_reports_nothing_to_bind"],
            ...Object.fromEntries(
                entries([21, 22, 23, 24], [S, "binds_clear_validates_payload_and_target (target errors)"]),
            ),
            28: [A, "binds_clear_validates_payload_and_target"],
            29: [A, "binds_clear_validates_payload_and_target"],
            30: [A, "binds_clear_validates_payload_and_target"],
            31: [A, "binds_clear_validates_payload_and_target"],
        },
    },
    "bridge.test.ts": {
        rust: "crates/z2m-rs/tests/bridge.rs",
        default: [C, "request handler ported, no dedicated assertion"],
        map: {
            1: [A, "start_publishes_bridge_topics"],
            2: [A, "start_publishes_bridge_topics"],
            3: [A, "start_publishes_bridge_topics"],
            7: [A, "start_publishes_bridge_topics"],
            18: [S, "invalid payload rejected before reaching the adapter"],
            29: [S, "device removal by string id"],
            30: [S, "device removal by object id"],
            19: [AD, "permit-join info republish after a coordinator call"],
            20: [AD, "permit-join info republish while stopping"],
            23: [C, "transaction echo in responses"],
            24: [C, "error responses carry the transaction"],
            25: [C, "format errors in responses"],
            26: [C, "coverage helper"],
            27: [A, "health_check_and_invalid_options"],
            28: [AD, "coordinator check needs a live adapter"],
            31: [A, "force_remove_device_without_adapter"],
            40: [A, "group_add_and_rename_device"],
            43: [A, "group_add_and_rename_device"],
            68: [A, "group_add_and_rename_device"],
            105: [A, "definitions_include_custom_clusters"],
            106: [A, "definitions_include_custom_clusters"],
            ...Object.fromEntries(
                entries([56, 57, 58, 72, 73, 76, 77, 78, 79, 107, 108, 109, 110], [
                    OM,
                    "approved OMITTED handler (AGENTS §5 Q5)",
                ]),
            ),
            ...Object.fromEntries(
                entries([91, 92], [AD, "backup/archive touches the coordinator"]),
            ),
            ...Object.fromEntries(
                entries([49, 50, 51, 52, 53, 54, 55, 74, 75], [
                    AD,
                    "interview/install code talks to the coordinator",
                ]),
            ),
            ...Object.fromEntries(
                entries(range(15, 22), [AD, "permit join talks to the coordinator"]),
            ),
            ...Object.fromEntries(
                entries(range(80, 90), [AD, "reporting configuration goes through the adapter"]),
            ),
        },
    },
    "configure.test.ts": {
        rust: "crates/z2m-rs/tests/configure.rs",
        default: [C, "startup/interview scheduling ported, adapter-backed"],
        map: {
            10: [A, "configure request success path"],
            11: [A, "unknown device response"],
            13: [A, "non-configurable device response"],
            14: [S, "invalid payload response"],
            4: [C, "disabled device guard"],
        },
    },
    "externalConverters.test.ts": {
        rust: null,
        default: [OM, "external JavaScript evaluation is forbidden (AGENTS §5 Q5)"],
    },
    "externalExtensions.test.ts": {
        rust: null,
        default: [OM, "external JavaScript evaluation is forbidden (AGENTS §5 Q5)"],
    },
    "frontend.test.ts": {
        rust: "crates/z2m-rs/tests/frontend.rs",
        default: [C, "frontend behaviour ported, no dedicated assertion"],
        map: {
            1: [A, "start/stop with defaults (TCP listener)"],
            2: [A, "start/stop without host"],
            3: [A, "unix_socket_frontend_serves_websocket"],
            4: [A, "tls_frontend_serves_websocket"],
            5: [C, "missing TLS config falls back to plain HTTP (build_tls_acceptor)"],
            6: [C, "missing TLS file logs and falls back (build_tls_acceptor)"],
            7: [A, "replay_retained_and_receive_messages"],
            8: [C, "generic serve() handles request/upgrade callbacks"],
            9: [ST, "device icon serving deferred to OpenWrt packaging"],
            10: [ST, "static assets deferred to OpenWrt packaging"],
            11: [A, "auth_token_is_enforced"],
            12: [ST, "static assets deferred to OpenWrt packaging"],
            13: [ST, "static assets deferred to OpenWrt packaging"],
            14: [ST, "static assets deferred to OpenWrt packaging"],
            15: [C, "settings/extension mismatch guard in start()"],
            16: [A, "disables serving when disable_ui_serving is set"],
        },
    },
    "groups.test.ts": {
        rust: "crates/z2m-rs/tests/groups.rs",
        default: [C, "group state derivation ported, converter/endpoint dependent"],
        map: {
            1: [A, "optimistic state fan-out"],
            2: [A, "identical optimistic states are deduped"],
            3: [A, "optimistic state fan-out"],
            10: [A, "optimistic state fan-out"],
            23: [AD, "add to group goes through the coordinator"],
            35: [A, "unknown group response"],
            36: [A, "unknown device response"],
            37: [C, "unknown endpoint response"],
            38: [A, "invalid payload response"],
            39: [S, "invalid payload for add/remove"],
            40: [S, "member state filtering"],
        },
    },
    "health.test.ts": {
        rust: "crates/z2m-rs/tests/health.rs",
        default: [C, "device health bookkeeping ported"],
        map: {
            1: [A, "publishes_bridge_health_payload"],
            2: [A, "publishes_bridge_health_payload (interval)"],
            5: [A, "reset_on_check_clears_stats"],
            3: [C, "init device health from leave"],
            4: [C, "init device health from network address change"],
        },
    },
    "homeassistant.test.ts": {
        rust: "crates/z2m-rs/tests/homeassistant.rs",
        default: [C, "discovery mapping ported, no dedicated per-device assertion"],
        map: {
            1: [C, "object_id derivation keeps type/object_id unique"],
            2: [C, "state_value_template guards every property access"],
            3: [S, "number entity SET guard"],
            4: [S, "presence check (not truthiness) in state_value_template"],
            5: [C, "entity_category cascade for config entities"],
            6: [C, "entity_category cascade for config entities"],
            7: [A, "applies_expose_metadata_and_null_name"],
            8: [A, "applies_expose_metadata_and_null_name"],
            9: [S, "discovers_numeric_binary_enum_and_switch_entities"],
            ...Object.fromEntries(
                entries(range(10, 35), [
                    A,
                    "extension::homeassistant::action_value_tests::parses_action_names_like_upstream",
                ]),
            ),
            36: [S, "discovery dedupe by topic+payload"],
            38: [A, "applies_device_option_overrides_and_null_filtering"],
            39: [A, "applies_device_option_overrides_and_null_filtering"],
            40: [A, "applies_device_option_overrides_and_null_filtering"],
            41: [A, "device_option_homeassistant_false_disables_discovery"],
            42: [A, "applies_device_option_overrides_and_null_filtering (temperature: null)"],
            46: [S, "siren mapping from the warning composite"],
            51: [CDB, "Bosch BTH-RA compatibility mapping lives in the converter DB"],
            54: [S, "user configuration applied after converter mapping"],
            55: [C, "overrideHaDiscoveryPayload errors are caught and logged"],
            56: [C, "overrideHaDiscoveryPayload receives entity options"],
            57: [CDB, "Bosch BTH-RM230Z exposes live in the converter DB"],
            68: [C, "homeassistant.discovery_topic is honoured by discovery_topic_for"],
            69: [C, "attributes output rejected in start()"],
            70: [C, "discovery_topic == base_topic rejected in start()"],
            73: [A, "adjust_message_copies_hue_and_saturation"],
            74: [S, "adjust_message copies only when all properties exist"],
            75: [S, "adjust_message copies only when color exists"],
            86: [A, "clears_discovery_on_entity_removed"],
            87: [S, "group removal resolves by ID"],
            88: [S, "rename clears and rediscovers (homeassistant_rename)"],
            89: [S, "rename path shared by groups"],
            90: [C, "homeassistant_rename=false keeps existing topics"],
            92: [A, "action_publishes_device_trigger_and_republication"],
            93: [A, "action_publishes_device_trigger_and_republication"],
            94: [S, "action republish on advanced.output=json"],
            95: [S, "action republish on advanced.output=json"],
            97: [A, "experimental_event_entities_prepare_wildcard_action_types"],
            98: [S, "endpoint attribute republish in on_publish_entity_state"],
            99: [S, "endpoint attribute republish skips light_type selects"],
            100: [C, "on_publish_entity_state returns early for groups"],
            101: [C, "on_discovery_message clears outdated configs"],
            109: [C, "bridge entities are published once (dedupe)"],
            110: [A, "bridge discovery assertions in discovers_numeric_binary_enum_and_switch_entities"],
            111: [C, "removed exposes clear their topics"],
            112: [C, "exposes coming back trigger rediscovery"],
            113: [C, "exposes_changed event triggers discover()"],
            114: [C, "legacy action sensor"],
            115: [C, "settings/extension mismatch guard"],
        },
    },
    "networkMap.test.ts": {
        rust: "crates/z2m-rs/tests/network_map.rs",
        default: [C, "network map rendering ported"],
        map: {
            1: [A, "raw network map output"],
            2: [A, "graphviz network map output"],
            3: [A, "plantuml network map output"],
            4: [A, "unsupported type error"],
            5: [C, "disabled devices filtered out"],
            6: [AD, "retry needs a live adapter"],
        },
    },
    "onEvent.test.ts": {
        rust: "crates/z2m-rs/tests/on_event.rs",
        default: [S, "start/stop and device event dispatch"],
        map: {
            1: [A, "start and stop"],
            2: [S, "device event dispatch"],
            3: [S, "startup is not blocked by a failing handler"],
        },
    },
    "otaUpdate.test.ts": {
        rust: null,
        default: [OM, "OTA is out of R1 (AGENTS §4)"],
    },
    "publish.test.ts": {
        rust: "crates/z2m-rs/tests/publish.rs",
        default: [CDB, "toZigbee converter behaviour from zigbee-herdsman-converters (R1 Q4)"],
        map: {
            25: [S, "group message fan-out through members"],
            28: [S, "group endpoint handling"],
            34: [S, "optimistic=false suppresses the optimistic publish"],
            35: [S, "filtered_optimistic suppresses the optimistic publish"],
            36: [S, "optimistic=false for groups"],
            ...Object.fromEntries(
                entries(range(37, 41), [S, "parse_message_variants (invalid topics)"]),
            ),
            42: [A, "set_and_get_dispatch_to_converters"],
            43: [S, "multi-endpoint get dispatch"],
            44: [S, "multi-endpoint cover get"],
            45: [C, "unknown endpoint error"],
            46: [A, "unsupported_device_is_rejected (no definition)"],
            47: [C, "unknown property endpoint error"],
            ...Object.fromEntries(
                entries([48, 49, 50, 59, 60], [S, "parse_topic_resolves_entity_and_endpoint"]),
            ),
            ...Object.fromEntries(
                entries(range(51, 58), [S, "set topic parsing variants"]),
            ),
            61: [S, "invalid payload is rejected"],
            100: [A, "unsupported_device_is_rejected"],
            110: [S, "unsupported_device_is_rejected (entity not found path)"],
        },
    },
    "receive.test.ts": {
        rust: "crates/z2m-rs/tests/receive.rs",
        default: [S, "receive pipeline covered by the behavioural tests"],
        map: {
            1: [A, "registered_converter_publishes_converted_payload"],
            2: [S, "endpoint-aware converter matching"],
            3: [S, "endpoint-aware converter matching"],
            4: [A, "tests/calibration.rs default_precision_is_applied"],
            6: [S, "legacy integration toggle"],
            7: [A, "debounce_publishes_once_with_debounce_reason_and_updates_cache"],
            8: [S, "no-payload messages do not bypass the debounce"],
            12: [S, "debounce/retain via device options"],
            13: [A, "payload_conflict_uses_debounce_ignore"],
            14: [S, "state cache not republished during debounce"],
            17: [A, "tests/calibration.rs applies_calibration_offset_and_precision"],
            18: [S, "0 precision truncates"],
            19: [S, "precision via device options"],
            20: [S, "device_options precision override"],
            21: [CDB, "SP600 voltage correction lives in the converter DB"],
            22: [S, "single publish for duplicate conversions"],
            23: [S, "single publish for duplicate conversions"],
            24: [S, "last_seen epoch"],
            25: [S, "last_seen ISO_8601"],
            26: [S, "last_seen ISO_8601_local"],
            29: [A, "definition_less_device_skips_message_and_publishes_last_seen"],
            30: [S, "command messages are ignored"],
            31: [S, "elapsed timing"],
            32: [A, "unsupported_cluster_publishes_last_seen_only"],
            33: [CDB, "SP600 per-version payloads live in the converter DB"],
            34: [C, "exposes_changed event on converter announce"],
        },
    },
};

const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));

const ORDER = Object.keys(CASES);
const rows = [];
const summary = [];

for (const file of ORDER) {
    const config = CASES[file];
    const result = report.testResults.find((entry) => entry.name.endsWith(file));
    if (!result) {
        throw new Error(`missing ${file} in ${reportPath}`);
    }
    const counts = {};
    for (const [index, assertion] of result.assertionResults.entries()) {
        const oneBased = index + 1;
        const resolved = config.map?.[oneBased] ?? config.default;
        const [klass, evidence] = resolved;
        counts[klass] = (counts[klass] ?? 0) + 1;
        rows.push({
            file,
            index: oneBased,
            title: assertion.title.replace(/\|/g, "\\|"),
            klass,
            evidence: evidence ?? "",
            rust: config.rust,
        });
    }
    summary.push({ file, total: result.assertionResults.length, counts, rust: config.rust });
}

const lines = [];
lines.push("# Extension per-case coverage (upstream test/extensions → Rust)");
lines.push("");
lines.push(
    "Generated by `tools/oracle/map-extension-cases.mjs` from the pinned vitest report.",
    "Regenerate after every upstream bump:",
);
lines.push("");
lines.push("```bash");
lines.push("cd .upstream/zigbee2mqtt");
lines.push(
    "node node_modules/vitest/vitest.mjs run --config ./test/vitest.config.mts \\",
);
lines.push("  --testTimeout=60000 --reporter=json --outputFile=/tmp/upstream_ext.json \\");
lines.push("  test/extensions");
lines.push("cd ../..");
lines.push(
    "node tools/oracle/map-extension-cases.mjs /tmp/upstream_ext.json docs/extension-test-cases.md",
);
lines.push("```");
lines.push("");
lines.push("Coverage classes:");
lines.push("");
lines.push("| Class | Meaning |");
lines.push("|---|---|");
lines.push("| `asserted` | a named Rust test asserts the same observable behaviour |");
lines.push("| `slice` | a broader Rust test covers part of the behaviour |");
lines.push("| `code` | ported and reachable, no dedicated Rust assertion yet |");
lines.push("| `converter-db` | depends on `zigbee-herdsman-converters` definitions, excluded from R1 (Q4) |");
lines.push("| `adapter` | needs a live coordinator/adapter, HIL pending |");
lines.push("| `static` | frontend static assets deferred to OpenWrt packaging |");
lines.push("| `omitted` | approved OMITTED (AGENTS §5 Q5) |");
lines.push("");
lines.push("## Summary");
lines.push("");
lines.push("| Upstream file | Cases | asserted | slice | code | converter-db | adapter | static | omitted | Rust tests |");
lines.push("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|");
for (const entry of summary) {
    const cell = (klass) => entry.counts[klass] ?? 0;
    lines.push(
        `| ${entry.file} | ${entry.total} | ${cell(A)} | ${cell(S)} | ${cell(C)} | ${cell(CDB)} | ${cell(AD)} | ${cell(ST)} | ${cell(OM)} | ${entry.rust ?? "—"} |`,
    );
}
lines.push("");

for (const file of ORDER) {
    const fileRows = rows.filter((row) => row.file === file);
    lines.push(`## ${file}`);
    lines.push("");
    lines.push("| # | Upstream case | Class | Evidence / reason |");
    lines.push("|---:|---|---|---|");
    for (const row of fileRows) {
        lines.push(`| ${row.index} | ${row.title} | \`${row.klass}\` | ${row.evidence} |`);
    }
    lines.push("");
}

fs.writeFileSync(outPath, lines.join("\n"));
const totals = rows.reduce((acc, row) => {
    acc[row.klass] = (acc[row.klass] ?? 0) + 1;
    return acc;
}, {});
console.log(`wrote ${outPath}: ${rows.length} cases`, totals);
