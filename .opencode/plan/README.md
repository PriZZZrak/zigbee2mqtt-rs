# План завершения zigbee2mqtt-rs

Статус: **план работ, не отчёт о завершённой реализации**. Аудит исходников: Rust
`07bb90f1012e41897ce1d30f79e45a5cfd01ecf2`. Приоритеты уточнены в разговоре
2026-11-04; технические ADR ниже остаются предложениями. Первые проверки среды
датированы 2026-11-03 UTC, итоговый refresh — 2026-11-04T09:08:33.742736+00:00.
Timestamps сохранены без подмены даты. Источники и ограничения:
[`AUDIT-20261104.md`](../../docs/parity/evidence/AUDIT-20261104.md).

## Что именно нужно получить

1. **R1 — первый эксплуатационный релиз:** OpenWrt **24.10.8**, **Routerich AX3000 v1**,
   заявленное пользователем общее место **54 МБ**, координатор **SONOFF Dongle-P**.
   Приоритет: перенос существующей сети из оригинала и обратно, Lytko «L101 NS»,
   существующий датчик температуры и выключатели Moes. Точные Zigbee modelID,
   manufacturerName, endpoints и firmware ещё нужно снять; торговое название не
   является fingerprint. RAM и свободное место пока неизвестны.
   Личные устройства — приоритетные fixtures/HIL для **общего** механизма,
   никаких отдельных персональных путей обработки или исключений из требований.
2. **S1 — полная совместимость Z-Stack с зафиксированным upstream:** все общие
   функции приложения, весь каталог, CC2652P и сохранённая поддержка CC2531.
   Пользователь снял обязательность аппаратной проверки CC2531 **для R1**.
   Это не удаляет CC2531 из общего объёма и не делает R1 завершением S1.
3. **G — глобальная совместимость:** все поддерживаемые pinned upstream адаптеры
   и оставшиеся общие контракты. Будущие радиаторные термостаты включаются в
   персональную матрицу после выбора моделей; до этого их поддержка UNKNOWN.

R1 — явно ограниченный релиз для проверенной сети. Его нельзя называть drop-in
заменой для любого пользователя, «90% Windfront» или «поддержкой 5426 устройств».

## Состояние выполнения

- **OPS-02 частично**: artifact **пересобран из текущего SHA** (5 734 976 B,
  sha256 `f221602d…`), **`.ipk` собран через OpenWrt SDK 24.10.8**
  (5 682 979 B, sha256 `db61ab90…`, installed 12 165 120 B с app-resolved Windfront 2.15.0,
  `conffiles` сохраняет config, payload `sstrip` объяснён, повторная сборка даёт
  тот же хэш). `check_ipk.py` валидирует пакет (9 тестов). Install/upgrade/
  downgrade **BLOCKED** на Routerich + Dongle-P; CI jobs не запускались;
  free-space/RAM бюджет и HIL не проверены.
- **STORAGE differential частично**: upstream `Database.write()` фикстура +
  z2m-style config; **9/9 PASS**, пины MATCH. Сравниваются identity/endpoints/
  interview/swBuildId/имена, **raw persisted entries** (clusters/binds/
  configuredReportings/meta/lastSeen), **Group/GreenPower**, а также
  corrupted/truncated/stale-tmp rewrite. STORAGE-009 — `VERIFIED`; общий
  backup/restore, power-loss и HIL — нет.
- **EVID-04 частично**: **6 реальных differential-фикстур** (SNZB-02 x3,
  TS0002 multi-endpoint, Tuya TRV report + convertSet) Node-vs-Rust проходят на
  **app-resolved herdsman 10.10.0**, все 4 пина MATCH. Нет MQTT/ZCL-wire/storage/
  frontend differential. Bridge **теперь конвертирует Tuya commandDataReport/
  commandDataResponse** через persistent session (CONVERTER-005): парсер
  `seq+dpValues`, fake-broker тесты проверяют опубликованный state, битый
  datapoint ничего не публикует. Tuya time-sync/gateway/weather теперь тоже
  структурированы (`payloadSize/payload`, Buffer для weather), OTA-команды
  явно отклоняются. **Bridge-level differential**: 7 фикстур (Tuya report,
  TS0002, SNZB-02 + time-sync/gateway/weather/OTA no-publish) сравнивают
  topic/payload реального bridge с pinned Node reference — 7/7 PASS.
  Differential нашёл и исправил двойной endpoint-суффикс и boolean-атрибуты.
  Ответные команды конвертеров (`msg.endpoint.command`: time sync, gateway
  status, weather) теперь **отправляются** в координатор через
  `forward_frame_commands`; AF-кадр проверен fake-coordinator тестом, а
  `fromZigbeeOps` differential сравнивает запрошенные команды, **read и write**
  с Node (4 кейса: Tuya gateway/weather, Leviton write, Yale read).
  Read/write из fromZigbee тоже пересылаются (`request_zcl`/`send_zcl`);
  добавлен `console`-шим (иначе logger ломал конвертеры) и `msg.meta.tsn`.
  Добавлен **ZCL-wire differential**: pinned herdsman `Zcl.Frame.create`
  кодирует те же команды/write/read, байты совпадают с Rust для 4 кейсов
  (gateway/weather/Leviton/Yale). **Read-результат теперь возвращается
  конвертеру**: `ZclIo` (coordinator Mutex + Tokio handle) выполняет
  `request_zcl` из converter-потока, пока bridge ждёт сессию; ZNP pump работает
  независимо, deadlock нет. Fake-coordinator multi-thread тест подтверждает
  `read_temperature: 2250`. **Write/command теперь ждут результат**: AF-status
  проверяется, при `disableDefaultResponse:false` — ZCL default response
  (success/failure/AF-status покрыты тестами).
- **Frontend/API differential расширен**: pinned upstream z2m vitest-dump
  (`bridge/state`, полный `bridge/info`, `bridge/devices` с coordinator-записью
  и `definition`/`exposes`/`options`, `permit_join` response) против реального
  Rust bridge — **4 кейса PASS + 1 явный known-gap**, пины MATCH. Реализованы:
  сетевые параметры координатора (`ZDO_EXT_NWK_INFO`), MQTT URI/version, OS,
  redacted config, `restart_required`, converter version; definition payload с
  upstream `prepareDefinition` (extend/action-merge/generic toZigbee/calibration/
  converter options/function exposes/white-label/linkquality), `supported =
  !!definition`, `description`/`date_code`. Найдены и исправлены: **Boa не имеет
  `String.prototype.substr`** → `Buffer.from(hex)` падал и 12 lumi-устройств не
  загружали definition; fallback каталога игнорировал `zigbeeModel` кандидата
  (registry теперь несёт `lookupCandidates`). Остаётся **генерация definitions**
  для неизвестных устройств **реализована** (CONVERTER-021): порт
  `generateDefinition` с полными endpoint-кластерами, `supported=false` для
  generated, публикация после join/failed interview; отдельный
  Node-vs-Rust differential — **5/5 PASS**, API-кейс теперь **5/5** без
  known-gap. Остаются `config_schema`, endpoint bindings/reportings, coordinator
  meta и HTTP/WS frontend.
- **Frontend HTTP/WS начат (FRONTEND-005)**: dependency-free HTTP/1.1 + RFC6455
  сервер (`src/frontend`), config `frontend`, token-auth (close 4401), replay
  retained + device state, forwarding клиентских сообщений в MQTT, раздача
  assets с `.br`/`.gz`, `no-store` для HTML и точной srvx 404-страницей.
  Клиентские сообщения **напрямую** парсятся тем же `commands_from_publish` и
  кладутся в bridge command queue (без broker echo), как upstream
  `mqtt.onMessage`. Pinned static differential расширен до **14/14 PASS**:
  index/extensionless, `.br`/`.gz`/preference, HEAD, ETag + Last-Modified,
  `If-None-Match`/`If-Modified-Since` → 304, `If-Range` (match 206, weak 200),
  byte ranges 206/416 и srvx 404. Rust integration tests покрывают
  auth/replay/live-forwarding/direct-dispatch/redirect; unit tests — ETag, даты,
  ranges и encodings; absolute `frontend.host` поднимает unix socket (общий
  HTTP/WS handler, тест с `UnixStream`). Остаются TLS, Node WS differential,
  fragmentation, live broker/HA и HIL.
- **PR-01 — выполнено локально** (`a700832`, `f4ef0f0`): baseline/evidence
  зафиксированы, CSV отремонтирован, неподтверждённые `VERIFIED` и case-счётчики
  переоткрыты, добавлены строгий валидатор, генерация dashboard с CI-check и
  Rust/parity workflow. GitHub Actions ещё не запускался.
- **PR-02 — частично выполнено**: STORE-01 (lossless YAML merge, unknown
  options, referenced files), STORE-02 (стабильные group/device database ID,
  epList/interviewState/swBuildId) и STORE-03 (SIGTERM-handler, принудительный
  flush, fsync каталога, видимые ошибки записи) реализованы с локальными
  regression tests. STORE-04 начат: offline `--inspect` и проверяемый
  `--snapshot-dir`; автоматический restore/rollback и реальный HIL-перенос ещё
  не сделаны. SIGTERM/ENOSPC/power-loss и Node/differential oracle отсутствуют,
  поэтому строки `IN_PROGRESS`, а не `VERIFIED`.
- Следующий шаг: завершить STORE-04 (restore/rollback), затем reference harness
  (EVID-04) и Z-Stack resume (ZB-01).
- **PR-03 начат (ZB-01, ZB-02, ZB-03, ZB-04)**: SREQ сериализуются FIFO;
  startup strategy resume/restore/commissioning с NV probe (resume не перезаписывает
  NV, restore отклонён); unknown NWK дедуплицируется; ZCL-транзакции получили waiter
  registry с match по адресу/endpoint/cluster/TSN/command, AF confirm, timeout и
  retry, но `request_zcl` ещё не используется bridge/converters. Recovery, route
  discovery и hardware tests отсутствуют.
- **CV-03 начат**: registry lookup по Zigbee modelID, bucket `null`, NUL-нормализация
  и fingerprint priority реализованы; product `models` больше не используется как
  ключ. Неизвестные fingerprint-поля, реальный пакет и Node oracle ещё не проверены;
  CV-04..08 не начаты.
- **CV-01/02 частично**: workflow сохраняет `devices/`, `converters/`, `lib/`,
  генерирует manifest с SHA-256 и валидирует пакет перед публикацией; добавлены
  Python layout tests. GitHub Actions и Node staged smoke ещё не выполнялись,
  live `latest` до следующего запуска остаётся в старом layout.
- **CV-04 частично**: registry-пути проверяются на traversal/absolute/backslash,
  загрузка ограничена 8 MiB. Digest/ETag, timeout, versioned cache и atomic
  promotion ещё не реализованы.
- **RT-03 частично**: `mqtts://` больше не превращается молча в plain TCP —
  конфигурация помечает TLS-требование, connect возвращает явную ошибку; URI
  сохраняется lossless. TLS provider и `mqtt.user` auth не реализованы.
- **RT-04 частично**: MQTT `get` теперь выполняет native ZCL reads через
  `request_zcl` (state/brightness/color/temperature/humidity/illuminance/battery/
  occupancy/contact), объединяет атрибуты по endpoint/cluster и публикует только
  реальные ответы; cache не выдаётся за ответ. Для не-native ключей исполняется
  upstream `convertGet` (Boa + read recording), его reads отправляются через
  `request_zcl`. Reporting read/configure реализованы через ZCL 0x08/0x09 и
  0x06/0x07, успешные configs сохраняются в `configuredReportings`. Broker-free
  тесты покрывают MQTT-команду, target mapping, read/parse/merge и полный
  fake-coordinator reporting E2E (0x08/0x09, 0x06/0x07, bind, persistence, reply
  envelope) и broker-free MQTT get/reporting pipeline с реальными topic/payload.
  Реальный broker/coordinator, retain/QoS/auth и HIL отсутствуют.
- **CV-08 частично**: async `convertGet` исполняется с Boa job queue и возвращает
  запрошенные чтения; stub поддерживает только `read`, endpoint всегда 1,
  timers/другие host methods не реализованы.
- **FEAT-02 OTA частично**: ZCL 0x0019 codecs, `OtaTransfer` и OTA manager
  (hex image header, bridge check/update/abort, incoming query/block/end,
  progress) покрыты fake-coordinator replay. Bridge request E2E для
  check/update/abort и progress payload добавлен; checksum SHA-512 и локальный
  index/sha512 подключены к bridge check/update; HTTP/override index provider и
  scheduler `check_for_update` покрыты fake-fetcher тестами; incoming
  QueryNextImage запускает auto-check и публикует `available` с throttling;
  bridge-level helper проверен fake index/device fixture. Полный Message loop,
  настраиваемый override, реальный HTTPS/cache и HIL отсутствуют.
- **CV-09 convertSet частично**: `toZigbee.convertSet` исполняется в persistent
  Boa session, записанные write/command/read кодируются по опубликованному
  `zcl-commands.json`; генератор **реально запущен** (129 типизированных
  кластеров, 4560 дефинишенов), layout-валидатор исправлен для `.d.ts`, реальный
  пакет конвертирует SNZB-02 идентично Node. Synthetic-модули для
  `zigbee-herdsman/dist/...`, `node:assert/buffer/util/fs`; regex-fingerprint
  (`ieeeAddr:{}`) больше не ломает загрузку реестра.
  Generator/Node не запускался; composite/Buffalo-типы (array/struct/time/date/
  IEEE/lists/zoneinfo/buffer/selector/BE-u24) добавлены, context-dependent типы
  отклоняются явно. Bridge fake-coordinator E2E проверяет точный AF-кадр для
  write, manufacturer command и read; отклонённый AF-запрос не блокирует очередь,
  `send_zcl` возвращает ошибку со статусом. Lock-команды ждут Lock/Unlock Door
  Response, optimistic state обновляется только при status 0. Опции
  (transition/executeIfOff/readAfterWriteTime) и timeout/retry не покрыты.
- **HA config switches/locks частично**: `SWITCH_DIFFERENT`-свойства дают
  config-switch с `command_topic_postfix`, lock discovery (0x0101) и cover
  `running`/`motor_state` шаблоны портированы с golden-тестами. Значения
  config-switch требуют `toZigbee.convertSet`, lock-ответ не ожидается, live
  broker/HA и HIL отсутствуют.
- **HA cover/postfix частично**: cover discovery (single/`cover_lN`), OPEN/
  CLOSE/STOP + position/tilt, per-attribute `set/<attr>` и `get/<attr>`,
  подписка `<base>/#`. Добавлен in-process MQTT broker: два теста гоняют
  реальный rumqttc event loop по TCP (команды + retained cleanup). Остаются
  SWITCH_DIFFERENT/lock postfix, cover quirks, live broker/HA и HIL.
- **HA multi-endpoint частично**: устройство с несколькими OnOff endpoint
  публикует `light_lN`/`switch_lN`, команды идут на `<friendly>/lN/set`,
  `state_lN`/`brightness_lN` разбираются, а stripped-снимок публикуется на
  `<base>/<friendly>/<lN>` как в z2m. Cover/postfix-attribute topics, live
  broker/HA и HIL отсутствуют.
- **HA cleanup частично**: retained discovery configs удаляются пустым payload
  (`retain=true`, QoS1) при remove; rename очищает старые configs и публикует
  новые. Fake MQTT sink E2E проводит device/remove через publish path и проверяет
  topics, пустой payload, QoS1/retain и response; полный socket event loop, live
  broker/HA migration и группа discovery не проверены.
- **FEAT-01 groups частично**: membership add/remove/remove_all отправляются
  реальными ZCL Groups-командами (0x0004) с ожиданием ответа и обновляют
  локальное состояние только после success; remove_all исправлен на
  endpoint-направление. Group fanout/state, bind flags и HIL отсутствуют.
- **CV-05/07/08 частично**: упрощённый `prepareDefinition` объединяет modern
  extend from/to/exposes/options; в `msg` передаются реальные endpoint/LQI/groupID
  и helpers `getDevice`/`deviceIeeeAddress`; native и converter значения
  сливаются в одну публикацию с приоритетом definition.
  Exposes API возвращает prepared array/function exposes и подключён к HA discovery
  для numeric/binary/switch/light (golden tests); climate/cover/lock/fan и cleanup
  не реализованы. meta/configure/onEvent и publish-callback semantics ещё не готовы.
  Persistent Boa context/timers реализованы как dedicated converter actor thread
  (ADR-08 вариант 1): module state, device-keyed store и due-timers переживают
  кадры. Async fz Promise ожидается, future-delay timers продвигаются fake-clock
  pump (до 60 s); wall-clock scheduling, eviction и RAM budget открыты.

## Документы

| Документ | Назначение |
|---|---|
| [01-current-state.md](01-current-state.md) | Оценка кода, подтверждённые дефекты, качество тестов |
| [02-parity-and-ci.md](02-parity-and-ci.md) | Достоверный baseline, инвентаризация, oracle, CI |
| [03-storage-and-config.md](03-storage-and-config.md) | Двусторонняя переносимость и сохранность состояния |
| [04-zstack-and-zigbee.md](04-zstack-and-zigbee.md) | Dongle-P, legacy CC2531, transport, interview, ZCL |
| [05-converters.md](05-converters.md) | Actions, registry, cache, Boa, host API, реальные устройства |
| [06-runtime-mqtt.md](06-runtime-mqtt.md) | Event loop, команды MQTT, состояние, reconnect, shutdown |
| [07-windfront-ha-and-features.md](07-windfront-ha-and-features.md) | WebSocket API, UI, HA, groups, scenes, OTA |
| [08-openwrt-and-release.md](08-openwrt-and-release.md) | 54 МБ, упаковка, измерения, миграция, rollout |
| [09-global-parity.md](09-global-parity.md) | Полный S1, остальные адаптеры, долгосрочная поддержка |
| [10-acceptance-matrix.md](10-acceptance-matrix.md) | Сценарии и обязательные доказательства приёмки |
| [11-decisions-and-questions.md](11-decisions-and-questions.md) | Принятые решения и оставшиеся вопросы |

## Очерёдность и зависимости

```text
A0: снимок исходников + исправление учёта + паспорт роутера/устройств
 ├─ A1: config/database round-trip ───────────────┐
 ├─ A2: безопасный resume Dongle-P + ZCL requests ├─ A5: Lytko/Moes/sensor end-to-end
 ├─ A3: reproducible Actions + registry + cache ─┤
 └─ A4: Boa real-package spike + host bridge ────┘
A5 + MQTT lifecycle + минимальный настоящий Windfront API
 → A6: OpenWrt package + recovery + HIL/soak → R1
 → общие группы/scenes/reporting/OTA/HA/config/frontend gaps → полный S1
 → адаптерные стадии + исчерпывающий аудит → G
```

Независимые ветки можно вести параллельно отдельными изменениями. Общие
контракты host API/ZCL requests согласовать до их интеграции. Никакой фоновой
бизнес-логики только ради параллелизма: один владелец состояния bridge.

### Ближайшие reviewable изменения

1. **PR-01 / P0:** `EVID-01..03`: валидатор inventory, реальная версия Windfront,
   доказательства вместо исторических процентов; Rust CI.
2. **PR-02 / P0:** `STORE-01..03`: воспроизводящие round-trip fixtures и сохранение
   неизвестных полей/ссылок YAML; стабильные ID database/group.
3. **PR-03 / P0:** `ZB-01..03`: resume/restore/commissioning и подтверждения ZCL;
   никаких экспериментов со сбросом пользовательской сети.
4. **PR-04 / P0:** `CV-01..03`: layout опубликованного пакета, version manifest,
   staged Node smoke и atomic latest; тест начинает с пустого каталога cache.
5. **PR-05 / P0:** `CV-04..06`: upstream lookup + prepareDefinition + real-package
   Boa spike. Выход — измеренный отчёт о возможностях, не очередной stub.
6. **PR-06 / P0:** `CV-07..09`: общий async host API, real fz/tz/configure;
   первыми проверяются фактический Lytko, затем Moes и датчик пользователя.
7. **PR-07 / P0:** `RT-01..04`: get/set payloads, lifecycle, durable rename/remove,
   устранение ложного успеха команд.
8. **PR-08 / P1:** `UI-01..03`, `HA-PLAN-01`: настоящий WebSocket API и минимальный
   UI/HA для acceptance-устройств, проверенный браузером.
9. **PR-09 / P0:** `OPS-01..06`: reproducible OpenWrt artifact, место/RAM, rollback,
   HIL и длительная работа; публикуется R1 с перечнем подтверждённых возможностей.

PR — логическая единица; не требование делать огромный коммит. Дробить по
независимым поведению и отрицательным сценариям. Не смешивать перенос storage,
рефакторинг event loop и новые converters в одном изменении.

## Общий контракт исполнения каждой задачи

- Перед кодом: refresh upstream, точный SHA/lockfile, source trace и исходные
  тесты. Указанные ниже источники — стартовые точки, не разрешение угадывать
  непрочитанные ветви.
- Task ID (`STORE-01`, `CV-01` и т.п.) — ID плана. Inventory IDs указаны в
  подразделах; широкий legacy row разбить до начала реализации, сохранив историю.
- Acceptance: одинаковые наблюдаемые данные/ошибки/порядок/retained/QoS,
  persistence и restart; компиляция и HTTP 200 отдельно от parity.
- При изменениях интерфейсов модульная декомпозиция сохраняет `src/lib.rs`,
  thin CLI, один bridge event loop, static `handler_for()`, ZNP actor и 10 s.
- `VERIFIED` только по требованиям AGENTS; HIL только там, где нужен hardware.
  Pure JSON преобразование можно доказать differential без координатора.
- В каждом PR: evidence, inventory, dashboard, тестовые артефакты и release notes
  о оставшихся отличиях. Непройденный gate блокирует соответствующую веху.

Календарный срок сейчас не обоснован: неизвестны real-package footprint Boa,
точные модели Moes/датчика, состояние coordinator backup и доступный RAM.
После A0/A4 оценивать оставшиеся работы по измеренным результатам, а не по
количеству Rust-файлов или строк CSV.
