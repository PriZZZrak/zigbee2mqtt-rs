// Generates the ZNP MT command catalog from the pinned upstream definition.
//
// Usage: tsx tools/oracle/gen-definition.mjs
//
// Writes:
//   crates/zigbee-apiarist/src/adapter/z_stack/znp/definition_data.rs
//   crates/zigbee-apiarist/tests/fixtures/definition.json

import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import DefinitionModule from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/znp/definition.ts";
import ParameterTypeModule from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/znp/parameterType.ts";
import {Subsystem, Type} from "../../.upstream/zigbee-herdsman/src/adapter/z-stack/unpi/constants.ts";

const Definition = DefinitionModule.default ?? DefinitionModule;
const ParameterType = ParameterTypeModule.default ?? ParameterTypeModule;

const OUT = fileURLToPath(
    new URL("../../crates/zigbee-apiarist/src/adapter/z_stack/znp/", import.meta.url),
);
const FIXTURE = fileURLToPath(
    new URL("../../crates/zigbee-apiarist/tests/fixtures/definition.json", import.meta.url),
);

const subsystemName = (value) => Subsystem[value];
const typeName = (value) => Type[value];
const parameterTypeName = (value) => ParameterType[value];

const flattenParameters = (parameters) =>
    (parameters ?? []).map((parameter) => ({
        name: parameter.name,
        parameterType: parameterTypeName(parameter.parameterType),
    }));

const fixture = [];
const lines = [
    "// Generated from zigbee-herdsman v11.2.0 by tools/oracle/gen-definition.mjs.",
    "// Do not edit by hand; regenerate when the upstream definition changes.",
    "",
];

for (const [subsystemValue, commands] of Object.entries(Definition)) {
    const subsystem = subsystemName(Number(subsystemValue));
    if (!subsystem) {
        throw new Error(`unknown subsystem '${subsystemValue}'`);
    }
    lines.push(`pub static ${subsystem}: &[MtCmd] = &[`);
    for (const command of commands) {
        const request = flattenParameters(command.request);
        const response = flattenParameters(command.response);
        const zdo =
            command.zdoClusterId === undefined
                ? "None"
                : `Some(${command.zdoClusterId})`;
        lines.push("    MtCmd {");
        lines.push(`        name: ${JSON.stringify(command.name)},`);
        lines.push(`        id: ${command.ID},`);
        lines.push(`        r#type: Type::${typeName(command.type)},`);
        lines.push(
            `        request: &[${request
                .map(
                    (parameter) =>
                        `MtParameter { name: ${JSON.stringify(parameter.name)}, parameter_type: ParameterType::${parameter.parameterType} as u32 }`,
                )
                .join(", ")}],`,
        );
        lines.push(
            `        response: &[${response
                .map(
                    (parameter) =>
                        `MtParameter { name: ${JSON.stringify(parameter.name)}, parameter_type: ParameterType::${parameter.parameterType} as u32 }`,
                )
                .join(", ")}],`,
        );
        lines.push(`        zdo_cluster_id: ${zdo},`);
        lines.push("    },");

        fixture.push({
            subsystem,
            name: command.name,
            id: command.ID,
            type: typeName(command.type),
            zdoClusterId: command.zdoClusterId ?? null,
            request,
            response,
        });
    }
    lines.push("];");
    lines.push("");
}

writeFileSync(`${OUT}/definition_data.rs`, lines.join("\n"));
writeFileSync(FIXTURE, `${JSON.stringify(fixture, null, 2)}\n`);
process.stdout.write(`definition: ${fixture.length} commands\n`);
