# 06. Runtime, MQTT и состояние

Inventory: RUNTIME-001/002, MQTT-001..009, AVAIL-001, RUNTIME-003, MQTT-010.
R1 P0: корректные get/set, lifecycle, bounded work, отказ от ложного успеха.

## Источники

App `controller.ts` startup/shutdown/publishEntityState, `mqtt.ts`, `state.ts`,
`extension/{receive,publish,bridge,availability,health}.ts`, `types/api.ts`;
tests controller, receive/publish/bridge/availability/health, mqtt mocks и clock
fixtures. Rust `bridge.rs`, `mqtt/mod.rs`, `mqtt/attribute_output.rs`, `main.rs`.
В audit прочитаны relevant receive/publish и начало availability; остальные
ветви требуется исследовать до соответствующей реализации.

## RT-01 — один владелец бизнес-состояния

1. Выписать каждый `tokio::spawn`: serial actor, MQTT I/O, interview retry,
   database debounce, timers. Для каждого — ownership, cancellation, termination,
   события и locks. Task не должна незаметно мутировать DeviceRegistry из side path.
2. Оставить business transition в одном bridge loop; I/O tasks возвращают
   typed completion events. Pending operation table связывает request, result
   и state transitions, чтобы ожидание response не блокировало его приём.
3. Snapshot needed fields → drop guard → await. Проверить downloader branch,
   interview и group mutation на повторное взятие DashMap guard одного shard.
4. Извлекать request handlers в `bridge/requests/*`, lifecycle/interview/state
   в небольшие подмодули library; не разносить business logic по независимым loops.
5. Ограничить payload/job/queue sizes и время синхронного JS. Считать backlog,
   drops, decode errors и pending-operation age в diagnostics.

**Gate:** deterministic event traces для одновременного report/set/reconnect/
rename; deadlock-free under backpressure; no stale action after device removal.

## RT-02 — startup, reconnect, shutdown и restart

- Start sequence: config/storage validity → broker/coordinator по reference
  ordering → publication retained snapshots и readiness. Не сообщать online
  раньше необходимой готовности; differentiate broker-down/coordinator-down.
- MQTT reconnect: resubscribe + bridge snapshots + conditional state/discovery,
  retaining/clean session/QoS согласно options. Очередь не публикует устаревшее
  действие повторно только потому, что broker вернулся.
- Coordinator disconnect: явное состояние и определённая recovery policy;
  закрытый event channel сейчас просто завершает main loop.
- SIGTERM/SIGINT/OpenWrt service stop: controlled shutdown, pending write flush,
  stop converter timers, offline state delivery/ack где применимо, disconnect.
- `bridge/request/restart`: actual restart lifecycle или корректная error
  outcome, response ordering и supervisor handoff; не просто сообщение в log.
- Startup failure после MQTT connect корректно завершает созданные tasks.

**Gate:** реальный broker + fake coordinator; broker/USB выключаются по отдельности,
команда restart действительно создаёт новый цикл; procd stop сохраняет последнюю
accepted metadata revision. Ошибка shutdown flush видна в exit/log evidence.

## RT-03 — MQTT transport/config contracts

1. Сопоставить все mqtt options schema: URI, user/password, auth-only варианты,
   keepalive, client_id, clean session, protocol, QoS, retain, TLS/certs.
2. `mqtts` нельзя превращать в plain TCP. Полный TLS должен удовлетворять pure-Rust
   правилу; provider/crate выбирать после target-tree и реального source audit.
   Пока TLS не реализован — явная ошибка unsupported, не silent downgrade.
3. Preserve JSON bridge/state; device-specific retain/qos и upstream defaults
   сравнить отдельно: нынешний publish_retained применяет QoS1/retain ко всему.
4. Topic parser: friendly_name со slash, endpoint/property postfix, base_topic
   prefix, malformed JSON/raw payload, empty payload и retained commands.
5. Credentials редактируются/логируются безопасно, referenced secrets сохраняются
   как references (STORE-01); schema serialization не раскрывает network key.

**Gate:** записанный broker traffic сравнивается по payload types, topics,
retain/QoS и ordering; failure/reconnect/auth tests, configured TLS rejection
без попытки plain TCP. Не писать тест, который сам создаёт expected JSON и
на этом заканчивается.

## RT-04 — реальные set/get и state pipeline

1. `MqttCommand::GetDevice` должен сохранить исходный payload/keys и target,
   вызвать converter `convertGet`/ZCL read. Cache publish — отдельная source-
   backed операция, не замена `get`.
2. Set: endpoint selection, converter ordering/deduplication и actual error
   feedback, result.state vs optimistic option; current native command formats
   не ломать без regression/evidence applicability.
3. Receive: configured options/calibration/precision, multiendpoint keys,
   action lifecycle, invalid values, timestamps, linkquality; postprocess before
   state persistence и публикации согласно upstream.
4. State cache: deep/shallow merging по исходнику, null/delete/transient action,
   cache_state flags, persistent state file, send-on-startup; restart не сбрасывает
   параметры, нужные converter store и optimistic state.
5. Output JSON/attribute/attribute_and_json сравнивать с frontend feed: UI получает
   entity JSON независимо от MQTT attribute mode (upstream frontend tests).
6. Debounce/throttle/filter/repeat/last_seen/elapsed — отдельные cases с fake clock,
   не «таймер вообще есть».

## RT-05 — request/response correctness

1. Составить endpoint matrix из `types/api.ts` и extension subscriptions:
   request schema, validation, operation, side effects, response, events.
2. Не stringify transaction произвольно: допустимый тип и optionality из API.
   Не отвечать `ok` на неизвестную сущность, ошибка USB/storage → error.
3. Rename updates config, state topic retained cleanup, devices list и HA;
   collision, same-name и IEEE canonicalization cases обязательны.
4. Remove: actual leave/force/block semantics, records, configs, groups,
   converter timers и discovery cleanup; итог должен пережить restart.
5. Options: schema validation/apply/save/restart-required semantics; не echo.
6. Пока endpoint отсутствует, transitional behavior честно documented и tested;
   это не parity-equivalent upstream operation и не VERIFIED row.

## RT-06 — availability/health/logging

Active vs passive, per-device timeouts, ping queue/backoff/jitter и caps,
reconnect retrieval, group availability, disabled devices и cleanup timers.
Timestamp in `response_time` сейчас не health response latency: схемы health
и coordinator_check нужно воспроизвести, включая реальные probes.

Structured logs должны показывать operation/device/error, без «успеха» после
silent catch. Rate limiting malformed traffic не скрывает счётчик ошибки.
Log rotation и disk budget важны на 54 МБ; debug payloads не писать безлимитно.
