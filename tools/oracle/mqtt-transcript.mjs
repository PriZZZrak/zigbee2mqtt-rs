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
import { createConnection, createServer } from "node:net";
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
              // A retained message delivered to a new subscription has the
              // retain flag set (MQTT 3.1.1 section 3.3.1.3).
              socket.write(Buffer.concat([Buffer.from([0x31]), encodeLength(body.length), body]));
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

function connectPacket(clientId) {
  const name = Buffer.from("MQTT");
  const body = Buffer.concat([
    Buffer.from([0, name.length]),
    name,
    Buffer.from([4, 0x02, 0, 60]), // protocol level 4, clean session
    Buffer.from([0, clientId.length]),
    Buffer.from(clientId),
  ]);
  return Buffer.concat([Buffer.from([0x10]), encodeLength(body.length), body]);
}

function subscribePacket(filter) {
  const body = Buffer.concat([
    Buffer.from([0, 1]), // packet id
    Buffer.from([0, filter.length]),
    Buffer.from(filter),
    Buffer.from([0]), // requested qos 0
  ]);
  return Buffer.concat([Buffer.from([0x82]), encodeLength(body.length), body]);
}

/// Connect as a brand-new subscriber and return the retained PUBLISH packets
/// the broker delivers for `filter` (upstream `mqtt.js` behaviour: a new
/// subscription receives retained `bridge/state offline`).
function readRetained(port, filter, waitMs = 800) {
  return new Promise((resolve) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    let buffer = Buffer.alloc(0);
    let timer = null;
    const received = [];
    const finish = () => {
      if (timer) clearTimeout(timer);
      timer = null;
      socket.destroy();
      resolve(received);
    };
    socket.on("connect", () => socket.write(connectPacket(`transcript-reader-${process.pid}`)));
    socket.on("error", finish);
    socket.on("close", finish);
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
        const type = buffer[0] >> 4;
        const flags = buffer[0] & 0x0f;
        const packet = buffer.subarray(index, index + length);
        buffer = buffer.subarray(index + length);
        if (type === 2) {
          socket.write(subscribePacket(filter));
          timer = setTimeout(finish, waitMs);
        } else if (type === 3) {
          const topicLength = packet.readUInt16BE(0);
          const topic = packet.subarray(2, 2 + topicLength).toString();
          const qos = (flags >> 1) & 0x03;
          let offset = 2 + topicLength + (qos > 0 ? 2 : 0);
          received.push({ topic, payload: packet.subarray(offset).toString(), qos, retain: (flags & 0x01) !== 0 });
        }
      }
    });
  });
}

function recordKey(record) {
  return `${record.topic}|${record.payload}|${record.qos}|${record.retain}`;
}

/// Order-sensitive, duplicate-preserving comparison (a reordering or a lost
/// duplicate is a mismatch, unlike the previous sorted-set comparison).
function compareRecords(rust, upstream) {
  const limit = Math.min(rust.length, upstream.length);
  let firstMismatch = null;
  for (let index = 0; index < limit; index++) {
    if (recordKey(rust[index]) !== recordKey(upstream[index])) {
      firstMismatch = { index, rust: rust[index], upstream: upstream[index] };
      break;
    }
  }
  const lengthMatch = rust.length === upstream.length;
  return {
    match: lengthMatch && firstMismatch === null,
    rustCount: rust.length,
    upstreamCount: upstream.length,
    firstMismatch,
  };
}

function compareSubscriptions(rust, upstream) {
  return {
    match: rust.length === upstream.length && rust.every((value, index) => value === upstream[index]),
    rust,
    upstream,
  };
}

function offlineIsDelivered(records) {
  return records.some(
    (record) =>
      record.topic === "z2m/bridge/state" &&
      record.payload === '{"state":"offline"}' &&
      record.retain === true,
  );
}

/// Guard the comparison itself: a comparator that ignores order, duplicates or
/// missing entries would let a regression through.
function selfTestComparators() {
  const base = [
    { topic: "a", payload: "1", qos: 0, retain: false },
    { topic: "b", payload: "2", qos: 0, retain: false },
  ];
  if (compareRecords(base, [base[1], base[0]]).match) {
    throw new Error("comparator did not detect a reordered publish");
  }
  if (compareRecords(base, [base[0]]).match) {
    throw new Error("comparator did not detect a lost publish/duplicate");
  }
  if (compareRecords(base, [...base, base[0]]).match) {
    throw new Error("comparator did not detect an extra publish");
  }
  if (compareSubscriptions(["a/#"], ["a/#", "a/#"]).match) {
    throw new Error("comparator did not detect a duplicate subscription");
  }
  if (compareSubscriptions(["a/#"], []).match) {
    throw new Error("comparator did not detect a missing subscription");
  }
  if (offlineIsDelivered([{ topic: "z2m/bridge/state", payload: '{"state":"online"}', retain: true }])) {
    throw new Error("offline detector accepted the online state");
  }
}

selfTestComparators();

function compare(rust, upstream, rustOffline, upstreamOffline, rustSubscriptions, upstreamSubscriptions) {
  const records = compareRecords(rust, upstream);
  const subscriptions = compareSubscriptions(rustSubscriptions, upstreamSubscriptions);
  const offlineSubscriber = {
    rust: offlineIsDelivered(rustOffline),
    upstream: offlineIsDelivered(upstreamOffline),
  };
  return {
    match: records.match && subscriptions.match && offlineSubscriber.rust && offlineSubscriber.upstream,
    records,
    subscriptions,
    offlineSubscriber,
  };
}

const rustBroker = await startBroker();
await runRust(rustBroker.port);
await sleep(300);
const rustRecords = summarize(rustBroker.records);
const rustSubscriptions = [...rustBroker.subscriptions];
// A brand-new subscriber must receive the retained offline state after the
// client stopped. Read it before closing the broker.
const rustRetained = await readRetained(rustBroker.port, "z2m/#");
rustBroker.close();

const upstreamBroker = await startBroker();
await runUpstream(upstreamBroker.port);
await sleep(300);
const upstreamRecords = summarize(upstreamBroker.records);
const upstreamSubscriptions = [...upstreamBroker.subscriptions];
const upstreamRetained = await readRetained(upstreamBroker.port, "z2m/#");
upstreamBroker.close();

const comparison = compare(
  rustRecords,
  upstreamRecords,
  rustRetained,
  upstreamRetained,
  rustSubscriptions,
  upstreamSubscriptions,
);
const transcript = {
  rust: { records: rustRecords, subscriptions: rustSubscriptions, retainedToNewSubscriber: rustRetained },
  upstream: {
    records: upstreamRecords,
    subscriptions: upstreamSubscriptions,
    retainedToNewSubscriber: upstreamRetained,
  },
  comparison,
};
mkdirSync(join(root, "docs/fixtures"), { recursive: true });
writeFileSync(join(root, "docs/fixtures/mqtt-transcript.json"), `${JSON.stringify(transcript, null, 2)}\n`);

console.log(`rust records: ${rustRecords.length}`);
for (const record of rustRecords) console.log(`  rust ^ ${record.topic} qos=${record.qos} retain=${record.retain} ${record.payload}`);
console.log(`upstream records: ${upstreamRecords.length}`);
for (const record of upstreamRecords) console.log(`  up   ^ ${record.topic} qos=${record.qos} retain=${record.retain} ${record.payload}`);
console.log(`subscriptions rust=${JSON.stringify(rustSubscriptions)} upstream=${JSON.stringify(upstreamSubscriptions)}`);
console.log(
  `new-subscriber offline rust=${comparison.offlineSubscriber.rust} upstream=${comparison.offlineSubscriber.upstream}`,
);
if (!comparison.match) {
  console.log("MISMATCH", JSON.stringify(comparison, null, 2));
  process.exit(1);
}
console.log(
  `OK: ${rustRecords.length} publishes and ${rustSubscriptions.length} subscription(s) match element-wise; retained offline received by a new subscriber`,
);
