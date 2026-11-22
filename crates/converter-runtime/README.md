# converter-runtime (prototype)

Minimal scheduler/runtime prototype for the post-R1 converter work. It is **not**
wired to Zigbee, MQTT, Home Assistant or the bridge.

- one quickjs-oxide worker thread;
- asynchronous Rust host API (`HostRequest` / `HostResult`) with correlation
  IDs;
- bounded per-device FIFO queues, round-robin fair devices, one active job per
  device and a global concurrency cap;
- response-driven Promise settlement (no automatic batch resolution);
- job lifecycle, timeout, cancellation, shutdown, late-response protection;
- per-job metrics: queue wait, eval, JS execution, Promise scheduling, host
  wait, total, GC.

See [`docs/converter-runtime-scheduler-prototype.md`](../../docs/converter-runtime-scheduler-prototype.md)
for measurements, golden parity, Routerich results and limits.

## Probe

```bash
cargo run -p converter-runtime --bin runtime_probe -- --mode all \
  --max-concurrent 4 --timeout-ms 60000
cargo run -p converter-runtime --bin runtime_probe -- --mode mix \
  --jobs 8 --max-concurrent 4 --host-delay-ms 1 --timeout-ms 30000
```

The probe answers host operations from a test responder; no serial port or
Zigbee command is used. Use `--host-mode drop|error|late`, `--queue-cap`,
`--total-cap`, `--cancel-after-ms` for failure tests.

Paths default to the local `.upstream` checkout and the study scenarios; on
Routerich pass `--package /opt/zigbee2mqtt/node_modules/zigbee-herdsman-converters`,
`--herdsman /opt/zigbee2mqtt/node_modules/zigbee-herdsman` and the staged
scenario/external paths.

## Known limits

- `submit` is non-blocking and bounded; host requests/responses are bounded and
  payload-limited.
- `setTimeout`/`clearTimeout`/`setInterval`/`clearInterval`/`.unref()` are
  implemented; timers may outlive a converter call and are cleared on
  cancellation/restart/shutdown.
- A shared-runtime JS timeout aborts all active jobs (single interpreter).
- In-process RSS/heartbeat supervision and `restart()` exist; an OS service
  supervisor is still required for process death.
- This is a scheduler prototype, not a deployment artifact.
