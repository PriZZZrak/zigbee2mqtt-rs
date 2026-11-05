import {afterAll, beforeAll, describe, it, vi} from "vitest";
import * as data from "../mocks/data";
import {events as mockMQTTEvents, mockMQTTPublishAsync} from "../mocks/mqtt";
import {flushPromises} from "../mocks/utils";
import {devices, returnDevices} from "../mocks/zigbeeHerdsman";
import fs from "node:fs";
import path from "node:path";
import {Controller} from "../../lib/controller";

const API_DATA_DIR = process.env.API_DATA_DIR as string;
vi.mock("../../lib/util/data", () => {
    const nodePath = require("node:path") as typeof import("node:path");
    const dir = process.env.API_DATA_DIR as string;
    return {
        default: {
            joinPath: (file: string): string => nodePath.join(dir, file),
            getPath: (): string => dir,
        },
    };
});

vi.mock("node:os", async (importOriginal) => ({
    ...(await importOriginal()),
    version: vi.fn(() => "Linux"),
    release: vi.fn(() => "0.0.1"),
    arch: vi.fn(() => "x64"),
    cpus: vi.fn(() => [{model: "Intel Core i7-9999"}]),
    totalmem: vi.fn(() => 10485760),
}));
vi.mock("node:process", async (importOriginal) => ({
    ...(await importOriginal()),
    version: "v1.2.3",
}));

describe("API dump", () => {
    let controller: Controller;

    beforeAll(async () => {
        fs.mkdirSync(API_DATA_DIR, {recursive: true});
        fs.writeFileSync(
            path.join(API_DATA_DIR, "configuration.yaml"),
            [
                "homeassistant:",
                "  enabled: false",
                "frontend:",
                "  enabled: false",
                "availability:",
                "  enabled: false",
                "mqtt:",
                "  base_topic: zigbee2mqtt",
                "  server: mqtt://localhost",
                "serial:",
                "  port: /dev/dummy",
                "advanced:",
                `  log_directory: ${API_DATA_DIR}`,
                "",
            ].join("\n"),
        );
        fs.writeFileSync(path.join(API_DATA_DIR, "state.json"), "{}");
        fs.writeFileSync(path.join(API_DATA_DIR, "database.db"), "");
        vi.useFakeTimers();
        controller = new Controller(vi.fn(), vi.fn());
        await controller.start();
        await flushPromises();
    });

    afterAll(async () => {
        await controller?.stop();
        await flushPromises();
        vi.useRealTimers();
    });

    it("dumps bridge MQTT payloads", async () => {
        const calls = mockMQTTPublishAsync.mock.calls as unknown[][];
        const latest = (topic: string): unknown => {
            for (let index = calls.length - 1; index >= 0; index -= 1) {
                if (calls[index][0] === topic) {
                    const raw = calls[index][1];
                    try {
                        return JSON.parse(String(raw));
                    } catch {
                        return String(raw);
                    }
                }
            }
            return null;
        };

        const before = calls.length;
        mockMQTTEvents.message("zigbee2mqtt/bridge/request/permit_join", JSON.stringify({time: 120}));
        await flushPromises();
        let permitJoin = null;
        for (let index = calls.length - 1; index >= before; index -= 1) {
            if (calls[index][0] === "zigbee2mqtt/bridge/response/permit_join") {
                permitJoin = JSON.parse(String(calls[index][1]));
                break;
            }
        }

        const deviceMeta = Object.values(devices)
            .filter((entry): entry is {ieeeAddr: string} => !!entry && typeof entry === "object" && "ieeeAddr" in entry)
            .map((entry) => {
                const device = entry as any;
                return {
                    ieeeAddr: device.ieeeAddr,
                    type: device.type,
                    nwkAddr: device.networkAddress,
                    manufId: device.manufacturerID,
                    manufName: device.manufacturerName,
                    powerSource: device.powerSource,
                    modelId: device.modelID,
                    interviewComplete: String(device.interviewState).toLowerCase() === "successful",
                    endpoints: (device.endpoints || []).map((endpoint: any) => ({
                        id: endpoint.ID,
                        profileId: endpoint.profileID,
                        deviceId: endpoint.deviceID,
                        inputClusters: endpoint.inputClusters || [],
                        outputClusters: endpoint.outputClusters || [],
                    })),
                };
            })
            .filter((entry) => entry.type !== "Coordinator" && entry.type !== "Group");

        const dump = {
            bridge_state: latest("zigbee2mqtt/bridge/state"),
            bridge_info: latest("zigbee2mqtt/bridge/info"),
            bridge_devices: latest("zigbee2mqtt/bridge/devices"),
            permit_join_response: permitJoin,
            device_meta: deviceMeta,
            return_devices: returnDevices,
        };
        fs.writeFileSync(process.env.API_DUMP_PATH as string, JSON.stringify(dump, null, 2));
    });
});
