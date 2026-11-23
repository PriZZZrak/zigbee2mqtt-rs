# zigbee2mqtt-rs

Rust-реализация Zigbee2MQTT для embedded ARM / OpenWrt. Цель — заменить
Node.js-мост, сохранив MQTT-контракты, сущности Home Assistant, API Windfront,
формат данных и операционные ожидания.

Текущий статус: **R1 (ядро) закрыт**. Тестовый роутер пока работает на
оригинальном Node.js Zigbee2MQTT; Rust-бинарник проверен через HIL, но
постоянным сервисом не развёрнут.

- R1 final tag: `r1-2026-11-20`
- R1 acceptance: [`docs/test-coverage-audit.md`](docs/test-coverage-audit.md)
  (CP-01…CP-12, все `CLOSED`)
- HIL: [`docs/hil/2026-11-18-final-r1-acceptance.md`](docs/hil/2026-11-18-final-r1-acceptance.md),
  [`docs/hil/2026-11-18-cp12-repeat.md`](docs/hil/2026-11-18-cp12-repeat.md),
  [`docs/hil/2026-11-19-db-format-parity.md`](docs/hil/2026-11-19-db-format-parity.md)
- Следующий этап: [`docs/converter-runtime-plan.md`](docs/converter-runtime-plan.md)
  (внешние конвертеры и полная база устройств)
- Node.js baseline конвертеров: [`docs/converter-benchmark-baseline.md`](docs/converter-benchmark-baseline.md)
  (harness и golden corpus — [`tools/bench/`](tools/bench/))
- Engine feasibility study: [`docs/converter-engine-study.md`](docs/converter-engine-study.md)
  (прототипы — [`tools/engine-study/`](tools/engine-study/))
- Async host validation: [`docs/converter-async-host-validation.md`](docs/converter-async-host-validation.md)
- Scheduler prototype: [`docs/converter-runtime-scheduler-prototype.md`](docs/converter-runtime-scheduler-prototype.md)
- Bridge integration contract: [`docs/converter-runtime-bridge-contract.md`](docs/converter-runtime-bridge-contract.md)
  (crate — [`crates/converter-runtime/`](crates/converter-runtime/))
- Offline bridge integration: [`docs/converter-bridge-integration.md`](docs/converter-bridge-integration.md)
  (dispatcher/tests — [`crates/z2m-rs/src/converter/`](crates/z2m-rs/src/converter/))
- Artifact research and manifest: [`docs/converter-artifact-research.md`](docs/converter-artifact-research.md),
  [`docs/converter-artifact-manifest.example.json`](docs/converter-artifact-manifest.example.json)

## Что входит в R1

- **ZNP / Z-Stack**: serial/TCP transport, UNPI framing, SREQ/SRSP matching,
  timeouts, start/resume без перезаписи NV, network parameters, permit join,
  soft reset/reconnect, firmware capabilities, backup/restore paths
  (destructive restore запрещён на живой сети).
- **Zigbee stack**: ZDO announce/address/descriptors/interview/bind; ZCL
  read/write/report/configure-reporting, default responses, группы,
  стандартные кластеры; controller model (Device/Endpoint/Group).
- **MQTT bridge**: `bridge/state`, `info`, `devices`, `groups`, `health`,
  `logging`, `definitions`; `bridge/request/*` для `options`, `permit_join`,
  `device/rename`, `device/remove`, `device/interview`, `health_check`,
  `networkmap`, `restart`; device `set`/`get`, транзакции, retained/QoS,
  reconnect и offline-state.
- **Home Assistant**: bridge entities, device-registry discovery, cleanup при
  rename/remove, availability. Per-device cluster discovery появляется вместе
  с конвертерами.
- **Frontend**: HTTP/WebSocket API и раздача Windfront из отдельного пакета
  (`ZIGBEE2MQTT_FRONTEND_ASSETS`), SPA fallback, TLS, auth.
- **Storage**: канонический `database.db` (NDJSON) и `configuration.yaml`
  (js-yaml-совместимый вывод), `state.json`, optimistic state, availability,
  last seen, restart/kill/reboot recovery.
- **Operations**: CLI, логирование, OpenWrt service, aarch64 musl,
  rollback-safe запись, импорт/миграции.

R1 **не включает** каталог устройств и `zigbee-herdsman-converters`,
пользовательский JS, OTA, сцены, другие адаптеры (ember/deconz/zigate/zboss/
zoh/ezsp), CC2531-приёмку и live-HA. Это задача следующего этапа.

## Проверки

Финальные software gates на R1-ревизии:

```text
cargo test --workspace                 80 suites, 1577 passed, 0 failed, 14 ignored
cargo clippy --workspace --all-targets exit 0 (warnings only)
cargo fmt --all -- --check             fails on 134 pre-existing files; all changed files formatted
node tools/docs/check-parity-maps.mjs  OK: 11 maps, 0 errors
model transcript                       16/16 match
MQTT transcript                        publishes/subscriptions match element-wise; offline retained received by a new subscriber
DB oracle                              Node -> Rust -> Node, 7081 bytes preserved
YAML/DB tests                          8 database + 3 YAML passed
fuzz_boundaries                        3 passed
```

Coverage: stable regions 73.33%, functions 75.06% (2609/3476), lines 73.11%;
nightly branches 72.56% (3593/4952). Точные команды и per-crate таблицы —
[`docs/coverage.md`](docs/coverage.md).

## Сборка

Требуется stable Rust, `cargo-zigbuild` и Zig для aarch64 musl.

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets

# aarch64 musl (пример)
PATH="/path/to/zig:$PATH" CC=zigcc CXX=zigcc \
  cargo zigbuild --release -p z2m-rs --target aarch64-unknown-linux-musl
```

Бинарник не требует Node.js. Внешние конвертеры (следующий этап) будут
исполняться встроенным JS-runtime; план и границы — в
[`docs/converter-runtime-plan.md`](docs/converter-runtime-plan.md).

## HIL

Аппаратная приёмка R1: Routerich AX3000 v1, OpenWrt 24.10.8 (aarch64),
SONOFF Dongle-P (CC2652P, Z-Stack3x0 20250321), живая сеть из 3 устройств.
Проверены resume без commissioning, MQTT bridge API, permit join, networkmap,
Windfront, HA bridge entities, сохранение `database.db`/`configuration.yaml`/
`state.json`, interview и отклик во время записи.

`state.json` после неудачного старта больше не очищается; формат БД/конфига
приведён к upstream и проверен побайтово (см. HIL 2026-11-19).

## Архив первой попытки

Первая реализация с Boa и converter-host отправлена в архив:

- ветка `archive/boa-js-host-20261106` (commit `502601f`);
- тег `archive-boa-js-host-20261106`.

Причина: per-device Boa-контексты, повторная загрузка модулей и синхронное
ожидание в event loop давали 6–9 с на definitions и 30–40 с до первой
команды на роутере. Архив используется как evidence, не как архитектура.

## Лицензия

Код проекта — MIT. Upstream Zigbee2MQTT/zigbee-herdsman-converters имеют
собственные лицензии; атрибуция и условия распространения бинарников и
пакета конвертеров должны быть подтверждены до релиза.
