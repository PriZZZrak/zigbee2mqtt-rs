#!/usr/bin/env node
// MQTT broker transcript: Rust client vs upstream zigbee2mqtt client (audit CP-02).
//
// Starts a minimal MQTT 3.1.1 broker, runs the same scenario with
//   A) the Rust `z2m-rs` Mqtt client (`cargo run --example mqtt_transcript`), and
//   B) the upstream `dist/mqtt.js` client from the pinned checkout,
// records every broker-side packet (topic, payload, qos, retain, order) and
// writes `docs/fixtures/mqtt-transcript.json` plus a pass/fail summary.
//
// Usage: node tools/oracle/mqtt-transcript.mjs
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const upstreamDist = join(root, ".upstream/zigbee2mqtt/dist");

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function encodeLength(length) {
  const bytes = [];
  do {
    let byte = length % 128;
    length = Math.floor(length / 128);
    if (length > 0) byte |= 0x80;
    bytes.push(byte);
  } while (length > 0);
  return Buffer.from(bytes);
}

function startBroker() {
  return new Promise((resolve) => {
    const records = [];
    const retained = new Map();
    const subscriptions = [];
    const server = createServer((socket) => {
      let buffer = Buffer.alloc(0);
      socket.on("data", (chunk) => {
        buffer = Buffer.concat([buffer, chunk]);
        for (;;) {
          if (buffer.length < 2) break;
          let index = 1;
          let length = 0;
          let multiplier = 1;
          let complete = false;
          while (index < buffer.length) {
            const byte = buffer[index++];
            length += (byte & 0x7f) * multiplier;
            if ((byte & 0x80) === 0) {
              complete = true;
              break;
            }
            multiplier *= 128;
          }
          if (!complete || buffer.length < index + length) break;
          const packet = buffer.subarray(index, index + length);
          const type = buffer[0] >> 4;
          const flags = buffer[0] & 0x0f;
          buffer = buffer.subarray(index + length);

          if (type === 1) {
            socket.write(Buffer.from([0x20, 0x02, 0x00, 0x00]));
          } else if (type === 8) {
            const pid = packet.subarray(0, 2);
            let i = 2;
            const filters = [];
            while (i + 2 <= packet.length) {
              const len = packet.readUInt16BE(i);
              i += 2;
              filters.push(packet.subarray(i, i + len).toString());
              i += len + 1;
            }
            subscriptions.push(...filters);
            socket.write(Buffer.from([0x90, 0x03, pid[0], pid[1], 0x00]));
            for (const [topic, payload] of retained) {
              const matches = filters.some(
                (filter) => filter === topic || (filter.endsWith("/#") && topic.startsWith(filter.slice(0, -1))),
              );
              if (!matches) continue;
              const body = Buffer.concat([Buffer.from([topic.length >> 8, topic.length & 0xff]), Buffer.from(topic), payload]);
              socket.write(Buffer.concat([Buffer.from([0x30]), encodeLength(body.length), body]));
            }
          } else if (type === 3) {
            const retain = (flags & 0x01) !== 0;
            const qos = (flags >> 1) & 0x03;
            const topicLength = packet.readUInt16BE(0);
            const topic = packet.subarray(2, 2 + topicLength).toString();
            let i = 2 + topicLength;
            let pid = null;
            if (qos > 0) {
              pid = packet.readUInt16BE(i);
              i += 2;
            }
            const payload = packet.subarray(i);
            records.push({ topic, payload: payload.toString(), qos, retain });
            if (retain) retained.set(topic, payload);
            if (pid !== null) socket.write(Buffer.from([0x40, 0x02, pid >> 8, pid & 0xff]));
          } else if (type === 12) {
            socket.write(Buffer.from([0xd0, 0x00]));
          }
        }
      });
      socket.on("error", () => {});
    });
    server.listen(0, "127.0.0.1", () => {
      resolve({
        port: server.address().port,
        records,
        subscriptions,
        close: () => server.close(),
      });
    });
  });
}

function runRust(port) {
  // Async spawn: the broker lives in this process, so a blocking spawnSync would
  // stop it from accepting the Rust client's connection.
  return new Promise((resolve, reject) => {
    const child = spawn(
      "cargo",
      ["run", "-q", "-p", "z2m-rs", "--example", "mqtt_transcript", "--", `mqtt://127.0.0.1:${port}`],
      { cwd: root, env: process.env, stdio: ["ignore", "pipe", "pipe"] },
    );
    let stderr = "";
    child.stderr.on("data", (chunk) => (stderr += chunk));
    const timer = setTimeout(() => child.kill("SIGKILL"), 300_000);
    child.on("exit", (code) => {
      clearTimeout(timer);
      if (code === 0) resolve();
      else reject(new Error(`rust example failed (${code}): ${stderr}`));
    });
  });
}

async function runUpstream(port) {
  const data = mkdtempSync(join(tmpdir(), "z2m-oracle-mqtt-"));
  writeFileSync(
    join(data, "configuration.yaml"),
    `homeassistant:\n  enabled: false\nmqtt:\n  base_topic: z2m\n  server: mqtt://127.0.0.1:${port}\nserial:\n  port: /dev/null\nadvanced:\n  log_level: warning\n`,
  );
  process.env.ZIGBEE2MQTT_DATA = data;
  const { createRequire } = await import("node:module");
  const require = createRequire(import.meta.url);
  const settings = require(join(upstreamDist, "util/settings.js"));
  settings.reRead();
  const logger = require(join(upstreamDist, "util/logger.js")).default;
  logger.init();
  const EventBus = require(join(upstreamDist, "eventBus.js")).default;
  const Mqtt = require(join(upstreamDist, "mqtt.js")).default;
  const mqtt = new Mqtt(new EventBus());
  await mqtt.connect();
  await sleep(500);
  await mqtt.publish("device/state", '{"state":"ON"}', { clientOptions: { retain: true, qos: 1 } });
  await mqtt.publish("device/plain", '{"plain":true}', { clientOptions: { retain: false, qos: 0 } });
  await sleep(2600);
  await mqtt.disconnect();
  await sleep(200);
}

function summarize(records) {
  return records.map((record) => ({
    topic: record.topic,
    payload: record.payload,
    qos: record.qos,
    retain: record.retain,
  }));
}

function compare(rust, upstream) {
  const key = (record) => `${record.topic}|${record.payload}|${record.qos}|${record.retain}`;
  const rustKeys = rust.map(key).sort();
  const upstreamKeys = upstream.map(key).sort();
  const missing = upstreamKeys.filter((entry) => !rustKeys.includes(entry));
  const extra = rustKeys.filter((entry) => !upstreamKeys.includes(entry));
  return { match: missing.length === 0 && extra.length === 0, missing, extra };
}

const rustBroker = await startBroker();
await runRust(rustBroker.port);
await sleep(300);
const rustRecords = summarize(rustBroker.records);
const rustSubscriptions = [...rustBroker.subscriptions];
rustBroker.close();

const upstreamBroker = await startBroker();
await runUpstream(upstreamBroker.port);
await sleep(300);
const upstreamRecords = summarize(upstreamBroker.records);
const upstreamSubscriptions = [...upstreamBroker.subscriptions];
upstreamBroker.close();

const comparison = compare(rustRecords, upstreamRecords);
const transcript = {
  rust: { records: rustRecords, subscriptions: rustSubscriptions },
  upstream: { records: upstreamRecords, subscriptions: upstreamSubscriptions },
  comparison,
};
mkdirSync(join(root, "docs/fixtures"), { recursive: true });
writeFileSync(join(root, "docs/fixtures/mqtt-transcript.json"), `${JSON.stringify(transcript, null, 2)}\n`);

console.log(`rust records: ${rustRecords.length}`);
for (const record of rustRecords) console.log(`  rust ^ ${record.topic} qos=${record.qos} retain=${record.retain} ${record.payload}`);
console.log(`upstream records: ${upstreamRecords.length}`);
for (const record of upstreamRecords) console.log(`  up   ^ ${record.topic} qos=${record.qos} retain=${record.retain} ${record.payload}`);
console.log(`subscriptions rust=${JSON.stringify(rustSubscriptions)} upstream=${JSON.stringify(upstreamSubscriptions)}`);
if (!comparison.match) {
  console.log("MISMATCH", JSON.stringify(comparison, null, 2));
  process.exit(1);
}
console.log("OK: rust and upstream broker traffic match");
