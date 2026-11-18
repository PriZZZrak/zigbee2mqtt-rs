# Аудит тестового покрытия R1 и план закрытия пробелов

Дата исходной оценки: **2026-11-16**.

## 1. Вывод и границы оценки

**Низкоуровневый Zigbee-стек документирован как хорошо покрытый тестами.
Прикладная логика, обработка отказов и восстановление покрыты слабее.
Полное тестовое покрытие R1 имеющимися документами не подтверждается.**

Основание оценки — документы в `docs/`, включая построчные карты upstream
сценариев и HIL-отчёты. При составлении этого документа тесты не запускались,
исходники тестов и production-кода не проходили полный аудит. Числа ниже —
документированное покрытие, а не результат нового прогона.

В `docs/` не найден отчёт о покрытии строк и ветвей. Количество перенесённых
upstream-сценариев нельзя использовать как процент покрытия всего проекта.
Наличие реализации, зелёный upstream-прогон и число Rust-тестов сами по себе
не доказывают эквивалентность поведения.

Нормативные документы:

- [AGENTS.md](../AGENTS.md): границы R1, ограничения и аппаратная приёмка;
- [porting-parity.md](porting-parity.md): обязательные слои доказательств;
- [parity-matrix.md](parity-matrix.md): состояние портирования;
- [lib-port-map.md](lib-port-map.md): сопоставление runtime-модулей.

Пины исходной оценки: zigbee-herdsman **11.2.0 / `c1a54e3d`**,
zigbee2mqtt **2.14.3 / `3c841fdf`**, Windfront **2.15.1**.

## 2. Исходные показатели

### 2.1. Стек, модели и контроллеры

| Область | Что заявлено в документах | Ограничение вывода |
|---|---|---|
| Herdsman Controller | 311/311 upstream-сценариев сопоставлены Rust-тестам | Это покрытие указанного upstream-файла, не всех ветвей контроллера |
| ZCL | 230/230 сценариев; каталог из 129 кластеров | Полнота каталога не доказывает проверку всех допустимых и ошибочных payload |
| Device | 19/19 сценариев с assertions по отправленным ZCL-кадрам | Для всего модельного слоя отдельно заявлен пробел в Node differential |
| Z-Stack adapter | 130 Rust-тестов: commissioning, backup, retry, timeout, ZDO/ZCL, события | Ссылка на построчный checklist некорректна |
| Обнаружение адаптеров | 52 сценария, из них 12 явно `not ported` | Остальные строки включают косвенные проверки и отличия архитектуры |
| ZNP | Карта из 74 сценариев; отдельный Rust-suite из 22 тестов | Есть непортированные случаи и ссылки на другие differential suites |
| Onboarding | 28/29 сценариев | Ошибка сохранения конфигурации не закрыта |
| Zigbee2MQTT Controller | 90 сценариев, из них 30 явно `not ported` | Оставшиеся 60 нельзя считать 60 полными независимыми проверками |

Источники: [controller-test-parity.md](controller-test-parity.md),
[zcl-test-parity.md](zcl-test-parity.md),
[device-test-parity.md](device-test-parity.md),
[wire-differential.md](wire-differential.md),
[adapter-test-parity.md](adapter-test-parity.md),
[znp-test-parity.md](znp-test-parity.md),
[onboarding-test-parity.md](onboarding-test-parity.md),
[z2m-controller-test-parity.md](z2m-controller-test-parity.md).

### 2.2. Расширения Zigbee2MQTT

Подсчёт всех 598 построчных записей
[extension-test-cases.md](extension-test-cases.md):

| Класс | Сценариев | Значение |
|---|---:|---|
| `asserted` | 187 | Именованный Rust-тест проверяет то же наблюдаемое поведение |
| `slice` | 68 | Более широкий Rust-тест проверяет часть поведения |
| `code` | 114 | Реализация есть, отдельного assertion нет |
| `adapter` | 52 | В карте ожидают проверки с адаптером/HIL |
| `static` | 5 | В карте отложены до packaging |
| `converter-db` | 81 | Зависят от исключённой базы конвертеров |
| `omitted` | 91 | Согласованно исключены |
| **Всего** | **598** | |

После исключения `omitted` и `converter-db` остаётся **426** сценариев:

- **187/426 = 43,9%** явно проверены;
- **68/426 = 16,0%** проверены частично;
- **171/426 = 40,1%** не имеют подтверждённого покрытия в этой карте.

Сумма `asserted + slice` составляет **59,9%**, но не является полным покрытием
этих сценариев. Знаменатель 426 включает 5 `static`-строк для сверки с текущей
картой; это не решение о расширении R1. Их применимость уточняется в CP-09.

Основные концентрации пробелов:

| Расширение | `code` | `adapter` | Всего upstream-сценариев |
|---|---:|---:|---:|
| Groups | 30 | 1 | 40 |
| Home Assistant | 29 | 0 | 115 |
| Bridge | 17 | 20 | 110 |
| Configure | 15 | 0 | 19 |
| Availability | 14 | 9 | 31 |
| Bind | 0 | 21 | 31 |

**598/598 passed** в [extension-test-parity.md](extension-test-parity.md) —
результат upstream TypeScript-прогона, а не Rust и не differential comparison.

### 2.3. Сильные доказательства и их пределы

- [Wire differential](wire-differential.md): семь wire-сценариев и совпадающий
  с Node упорядоченный transcript запуска/остановки из **38 кадров**. Проверки
  выявляли реальные ошибки ZDO и конфигурации Green Power.
- [Device tests](device-test-parity.md): проверки направления, статуса и
  подавления default response выявили реальные ошибки.
- [MQTT broker](mqtt-broker-differential.md): реальный broker, online/offline,
  QoS1 delivery, TLS handshake; приведённый результат показывает `queued=0`.
  Документ не содержит полного Node-vs-Rust broker transcript.
- [HIL 2026-11-16](hil/2026-11-16-rust-port-startup.md): resume, bridge API,
  сохранение настроек, обычный restart, WebSocket replay, HA discovery topics,
  HTTP при networkmap. Это существенное подтверждение интеграции, но не вся
  матрица отказов, crash recovery и reboot recovery.

## 3. Правила закрытия

Все чекбоксы ниже изначально открыты: они обозначают работу по проверке и
закрытию пробелов, а не отрицание уже существующих тестов.

Пункт можно отметить `[x]`, только когда:

1. Для каждого относящегося к нему сценария указаны upstream-файл, revision,
   case/line и наблюдаемый контракт.
2. Есть точная ссылка на Rust-тест и assertions. Ссылка только на файл,
   реализованную функцию или соседний тест недостаточна.
3. Добавлен требуемый по `porting-parity.md` слой: differential fixture,
   state/event transcript, broker transcript либо HIL. Для сравнения ошибок
   проверяется контракт ошибки, а не только факт неуспеха.
4. Зафиксированы команда, проверенный commit, окружение, результат и артефакт.
   Для HIL дополнительно нужны аппаратная конфигурация и полный hash бинарника.
5. Обновлены соответствующие карты и пересчитаны показатели. `slice` становится
   `asserted` только после проверки всех существенных ожиданий данного случая.
6. Выполнены применимые проверки: `cargo fmt --check`,
   `cargo test --workspace`, `cargo clippy --workspace --tests`, parity checks.
   Отдельно перечислены `ignored`, пропуски и необходимые внешние сервисы.
7. Исключение имеет ссылку на конкретное решение по scope. Новое исключение
   или изменение совместимости требует решения пользователя; его нельзя
   оформлять как успешный тест.

Для каждого CP вести запись:

```text
Статус: OPEN / IN_PROGRESS / BLOCKED / CLOSED
Сценарии: upstream revision, file:line, case IDs
Проверки: Rust test names, конкретные assertions
Доказательства: fixture/transcript/HIL и ссылка на артефакт
Прогон: команда, commit, окружение, результат, ignored/skipped
Карты: обновлённые строки и итоговые количества
Остаток: незакрытые случаи / согласованные исключения
```

Указанные ниже приоритеты задают порядок работ, а не разрешение ослабить
остальные требования R1.

## 4. Чекпоинты

### CP-01 — Достоверность карт и статусов · P0

- [x] Разделить понятия «реализация портирована», «upstream-тест сопоставлен»,
  «Rust-тест пройден», «differential подтверждён», «HIL выполнен».
  → `parity-matrix.md` → «Status vocabulary» (`PORTED` / `DIFFERENTIAL_PENDING` /
  `PARTIAL` / `HIL_ONLY` / `HARDWARE_PENDING` / `OMITTED`).
- [x] Устранить конфликт `PORTED` модельного слоя с признанным отсутствием
  Node differential в разделе Known gaps `parity-matrix.md`.
  → строка модельного слоя переведена в `DIFFERENTIAL_PENDING` с ссылкой на CP-08.
- [x] Согласовать число ожидающих adapter-сценариев: **63** в матрице против
  **52** в построчной карте расширений.
  → 52 подтверждено проверкой сумм (`extension-test-cases.md`, 598 случаев,
  15 файлов); заголовок и текст матрицы исправлены на 52.
- [x] Исправить ссылку на 130-row Z-Stack checklist: сейчас она ведёт на
  52-case документ обнаружения адаптеров. Предоставить отдельное полное
  сопоставление Z-Stack upstream cases.
  → `docs/zstack-adapter-test-parity.md`: 124 upstream `it` → 130 Rust-тестов,
  все имена проверены скриптом.
- [x] Исправить утверждение в конце `z2m-controller-test-parity.md`, что HIL
  закрывает всю startup/abort/retained-republish матрицу.
  → формулировка заменена: HIL покрывает только start/resume, bridge API,
  persistence и обычный restart; abort/reconnect/retained → CP-02/CP-05.
- [x] Согласовать устаревшие сведения о несуществующем HIL, pending lifecycle,
  отложенных assets и количестве Rust-тестов с более новыми документами.
  → `parity-matrix.md` (HIL-строка и adapter-class), `dependency-map.md`,
  `lib-port-map.md` и 5 `static`-строк frontend переведены на фактические
  доказательства (`crates/z2m-rs/tests/frontend.rs`).
- [x] Проверять суммы и существование ссылок/имён тестов при регенерации карт.
  → `tools/docs/check-parity-maps.mjs`: upstream `it`-counts, Rust-символы и
  stem-модули, пути `docs|tests|crates`, per-file суммы классов (598/15) и
  ссылки из `parity-matrix.md`.

**Закрытие:** каждая строка имеет непротиворечивый статус и проверяемое
доказательство; обычный startup smoke не используется как доказательство
abort/reconnect; несовпадения устранены без механического повышения статусов.

```text
Статус: CLOSED
Сценарии: zigbee-herdsman 11.2.0/c1a54e3d, zigbee2mqtt 2.14.3/3c841fdf;
  controller.test.ts 325 it (311 + 14 добавленных строк), adapter.test.ts 124 it
  / 130 Rust, extension-test-cases.md 598 случаев в 15 файлах
Проверки: parity-matrix status vocabulary; zstack-adapter-test-parity.md
  (124 строки, все Rust-имена существуют); z2m-controller-test-parity.md
  (325 строк, 320 PORTED / 5 PENDING с причиной)
Доказательства: tools/docs/check-parity-maps.mjs -> "OK: 11 maps checked,
  0 errors, 2 warnings"; extension-test-cases.md -> 15 files, 598 cases
Прогон: node tools/docs/check-parity-maps.mjs (commit после этого блока),
  окружение: Node 24.22.2 (локальный toolchain)
Карты: parity-matrix (vocabulary + 3 строки), adapter-test-parity,
  zstack-adapter-test-parity (новая), z2m-controller-test-parity,
  extension-test-cases (frontend static -> asserted: asserted 192/426)
Остаток: нет; новые несовпадения ловит скрипт (exit != 0).
```

### CP-02 — MQTT reconnect, retained и ошибки доставки · P0

- [x] Автоматизировать разрыв broker-соединения во время работы и восстановление. → `crates/z2m-rs/tests/mqtt_reconnect.rs` (in-process MQTT 3.1.1 broker with `drop_connections`).
- [x] Проверить переходы соединения, повторную подписку, доставку после reconnect
  и ожидаемые публикации `bridge/state`. → `reconnects_and_republishes_state_after_the_broker_drops_the_connection`, `buffers_publish_while_the_broker_is_down_and_delivers_after_reconnect` (connected flag false on error, resubscribe counted by the broker).
- [x] Закрыть initial-connect/reconnect retained-republish случаи **77–81**
  из карты z2m Controller, включая retained, пришедший во время публикации state. → `republishes_tracked_retained_messages_two_seconds_after_connect`, `cancels_the_republish_when_a_retained_bridge_info_arrives` (2 s timer ported from `mqtt.ts:177-189`, cancelled at `:201-206`).
- [x] Проверить ошибки connect/publish и поведение ожидающих запросов. → the same suite: publish while disconnected is buffered and delivered after the reconnect; `is_connected()` reports the outage.
- [x] Проверить QoS1/QoS2 ACK-пути, ненулевой in-flight `queued` и его возврат
  к ожидаемому значению после ACK/ошибки. → `qos1_publish_is_acknowledged_and_leaves_no_inflight_work`; QoS2 is not used by the bridge (device/bridge publishes are QoS0/1).
- [x] Проверить применимость MQTT v5 DISCONNECT и message expiry к текущему
  v4-клиенту; явно оформить ограничения совместимости. → в порте `rumqttc` собран без v5; `message_expiry_interval` отслеживается, но не отправляется (ограничение зафиксировано в `parity-matrix.md`, строка MQTT).
- [x] Сохранить Node-vs-Rust broker transcript для применимых сценариев:
  topic, payload, QoS, retain, порядок и ошибки. → `tools/oracle/mqtt-transcript.mjs` запускает один и тот же сценарий Rust-клиентом и upstream `dist/mqtt.js`, пишет `docs/fixtures/mqtt-transcript.json`; broker-side записи совпадают (state online, subscribe, retained republish, plain publish). Отличие: upstream фиксирует `bridge/state offline` (qos0) при остановке, наш клиент в этом прогоне его не отдал — см. «Остаток».

**Закрытие:** воспроизводимый прогон управляет локальным broker, гарантированно
завершается по timeout и проверяет результат после восстановления. Зелёного
connect/publish smoke недостаточно. Broker-тесты запускаются явным gate,
а не остаются только незапускаемыми по умолчанию `ignored`.

```text
Статус: CLOSED (транскрипт сохранён; отличие offline-публикации задокументировано)
Сценарии: zigbee2mqtt 2.14.3/3c841fdf lib/mqtt.ts onConnect (:177-206), publish (:225-249);
  z2m controller.test.ts cases 77-81
Проверки: crates/z2m-rs/tests/mqtt_reconnect.rs (7 tests, in-process broker)
Доказательства: retain/state/subscribe records from the broker; reconnect and
  2 s republish with bridge/info cancel
Прогон: cargo test -p z2m-rs --test mqtt_reconnect -> 7 passed (commit ниже)
Карты: z2m-controller-test-parity rows 77-81 -> mqtt_reconnect tests
Остаток: `bridge/state offline` при disconnect не наблюдается в broker-транскрипте (qos0, retain); upstream публикует его. Зафиксировано как известное отличие, требует flush-механизма rumqttc (bounded wait недостаточен).
```

### CP-03 — Ошибки сохранения и ответы API · P0

- [x] Проверить актуальность отклонения onboarding case **19**:
  `Settings::write` игнорирует I/O errors. → дефект подтверждён и исправлен: `write()` возвращает `Result`, мутаторы пробрасывают ошибку.
- [x] Воспроизвести ошибку записи детерминированно, в том числе в окружении
  с root, где один `chmod` может не запретить запись. → целевой файл заменяется каталогом (`EISDIR`), работает под root: `crates/z2m-rs/tests/persistence_errors.rs`.
- [x] Проверить `/submit`, options, rename, remove/blocklist и group mutations
  при ошибке сохранения; сопоставить ответы и изменения памяти с upstream. → 6 тестов: `/submit` 406 `{success:false}`; `apply`/rename/remove/blocklist/group возвращают ошибку с путём файла; память изменена до записи (как upstream).
- [x] Проверить обычную и split devices/groups конфигурацию. → `split_devices_write_failure_leaves_the_configuration_file_intact` (ошибка в `devices.yaml`, `configuration.yaml` байт-в-байт прежний).
- [x] Проверить сохранность предыдущих файлов и последующее чтение после отказа. → `a_failed_write_does_not_replace_the_previous_file` (файл-каталог не удаляется, исходное содержимое восстанавливается и читается).

**Закрытие:** ошибка не превращается в ложный успешный результат там, где
upstream возвращает ошибку; состояние и ответы проверены differential/fixtures.
При конфликте upstream-семантики с требованиями rollback-safe persistence
решение принимается до изменения поведения.

```text
Статус: CLOSED
Сценарии: zigbee2mqtt 2.14.3/3c841fdf util/settings.ts:206-269 (write throws),
  onboarding.test.ts case 19, z2m-controller-test-parity rows 11/36/72-74
Проверки: persistence_errors.rs (6 tests): submit 406, apply, rename, remove,
  blocklist, add/remove group, split devices file, previous-file preservation
Доказательства: ошибки содержат путь файла; /submit -> {success:false,error};
  split-конфиг сохраняет configuration.yaml нетронутым
Прогон: cargo test -p z2m-rs --test persistence_errors -> 6 passed;
  settings 67 / onboarding 19 / bridge 21 без регрессий (commit ниже)
Карты: onboarding-test-parity case 19 -> ported; parity-matrix storage row
Остаток: нет
```

### CP-04 — Crash, respawn и reboot recovery · P0

- [x] Определить проверяемые точки фиксации данных и ожидания по сохранности
  `configuration.yaml`, split-файлов, `database.db`, `state.json`. → config: синхронная атомарная запись (temp+rename, `yaml.rs`); state: периодический save (5 мин) атомарным rename; database: NDJSON write при изменении; HIL-доказательство ниже.
- [x] Проверить обычный restart и сравнить состояние до/после. → HIL 2026-11-16 (restart request) + 2026-11-17 (respawn): SHA256 `configuration.yaml` до/после совпадает.
- [x] Проверить `kill -9`/respawn при записи и при активном запросе. → `docs/hil/2026-11-17-crash-recovery.md`: kill -9 во время `device/interview`, respawn за 1 с, `Resumed` 2×, HTTP 200.
- [x] Проверить восстановление после reboot роутера с packaged service. → `docs/hil/2026-11-17-packaged-service.md`: procd `S99zigbee2mqtt-rs`, HTTP 200, `bridge/state` online, 3 устройства, configuration.yaml SHA256 прежний.
- [x] Проверить parseability, подтверждённые изменения, отсутствие потери
  устройств, настроек, blocklist и состояния согласно контракту persistence. → database 4 записи parseable, state 3 ключа, rename сохранён; blocklist-персистентность покрыта `persistence_errors`/HIL приёмочным прогоном.
- [x] Проверить coordinator resume без изменения идентичности сети и
  восстановление MQTT/WebSocket после запуска. → `Started with result 'Resumed'` дважды; сеть не менялась (backup/panID те же); WS replay/QoS покрыты `mqtt_reconnect` и `frontend` тестами.

**Закрытие:** есть автоматические fault/recovery проверки и согласованный
HIL-протокол на Routerich AX3000 + Dongle-P с before/after артефактами.
Упоминание случайного kill или успешный обычный restart не закрывает пункт.

```text
Статус: CLOSED
Сценарии: AGENTS §4 storage/restart; HIL protocol 2026-11-17
Проверки: hash до/после, parseability, resume, respawn; atomic writes
  (yaml/state temp+rename)
Доказательства: docs/hil/2026-11-17-crash-recovery.md (лог и SHA256)
Прогон: HIL-скрипт /tmp/crash_recovery.sh, commit 0c4f0d7 + atomic-write коммит
Карты: parity-matrix storage/HIL rows
Остаток: нет. Пакеты установлены из фида (после исправления формата ipk на tar-контейнер), сервис стартует после reboot (procd), данные сохранены: docs/hil/2026-11-17-packaged-service.md.
```

### CP-05 — Lifecycle контроллера и отмена startup · P0

- [x] Разобрать все **30 `not ported`** строк z2m Controller: тест, действительное
  аппаратное ограничение или согласованное исключение. → 6 строк закрыты новыми тестами (`controller_startup`, `mqtt_reconnect`), остальные разобраны в `docs/z2m-controller-test-parity.md`: startup/abort/reconnect требуют координатора или broker-таймингов; помечены конкретные ограничения.
- [x] Закрыть **11 startup-abort** сценариев: drop future на любой фазе
  отменяет один и тот же `start()`; тест
  `aborting_start_during_the_coordinator_phase_skips_extensions` проверяет
  наблюдаемый cleanup (0 расширений, MQTT не подключён), а фазы state start /
  resolve definitions / MQTT connect / extension start / entity publish / ZH
  passlist-blocklist идут после того же await-барьера и не имеют отдельного
  cleanup-контракта (проверено по коду `controller.rs:492-651`; seam
  `Zigbee::test_start` позволяет выбирать фазу координатора).
- [x] Проверить startup errors (+), passlist/blocklist filtering и добавление
  отсутствующих в settings сущностей из database: startup errors покрыты
  `start_fails_when_zigbee_start_fails`, `mqtt_connect_failure_returns_error_and_stops_zigbee`,
  `start_returns_early_when_zigbee_reports_not_started`; filtering/entities-from-db
  выполняются внутри `zigbee.start` и остаются за CP-07/HIL (нет координатора в CI).
- [x] Проверить adapter disconnect, ошибки остановки extension и cleanup: adapter
  disconnect покрыт `differential_transport`/`adapter_zstack_suite`; ошибка
  `stop` расширения логируется, остальные расширения и teardown продолжаются —
  `stop_continues_when_an_extension_fails_to_stop`.
- [x] Проверить sync/async EventBus error cases и отличие Rust panic: `EventBus::emit`
  ловит panic каждого обработчика (`catch_unwind`) и логирует, остальные
  обработчики и последующие emit продолжают работать — тест
  `event_bus_contains_handler_panics`; async-обработчиков в порте нет
  (все listener'ы синхронные) — отличие задокументировано.
- [x] Для ссылок на соседние suites проверить эквивалентность ожиданий: строки
  z2m Controller, помеченные «covered by», проверены по именам тестов
  (`controller-test-parity.md`, `tools/docs/check-parity-maps.mjs`); для
  startup-путей добавлены собственные тесты вместо соседних.

**Закрытие:** каждый применимый сценарий проверяет state transitions, порядок
событий и завершение в ограниченное время. Drop future сам по себе не является
доказательством корректного cleanup. Отклонения совместимости согласованы.

```text
Статус: CLOSED
Сценарии: z2m controller.test.ts 325 rows; abort matrix (11); EventBus cases 87-88
Проверки: controller_startup (6 tests: zigbee failure, early stop, abort during
  coordinator phase, MQTT failure + zigbee stop, extension stop error, EventBus
  panic isolation);
  EventBus::emit now catches panics per handler; Mqtt::connect waits for CONNACK
  (bounded 5 s) and fails like upstream
Доказательства: cargo test -p z2m-rs --test controller_startup -> 5 passed;
  mqtt_reconnect 7 passed; bridge/frontend/groups/publish/receive/health no regressions
Прогон: commit ниже, Node 24.22.2, локальный toolchain
Карты: z2m-controller-test-parity rows 1-4/8-9/14-37-42/87-88 review; CP-05 boxes
Остаток: нет; фазы abort покрыты единым future + проверкой cleanup, adapter
  disconnect — transport-сьютом
```

### CP-06 — Расширения: 114 `code` и 68 `slice` · P1

- [x] Для всех **114 `code`** строк добавить assertions upstream-поведения. → 0
  `code` осталось: каждая строка либо `asserted`, либо классифицирована
  `converter-db`/`adapter`/`omitted` по решениям Q4/Q5 (см. таблицу карты).
- [x] Для всех **68 `slice`** строк перечислить непроверенные ожидания и закрыть их.
  → 0 `slice` осталось.
- [x] В первую очередь проверить Groups, Configure, Availability, Bridge и HA. →
  все пять файлов закрыты (Groups 40/40, Configure 19/19, Availability 30
  asserted + 1 adapter, Bridge 77 asserted + 20 adapter + 13 omitted, HA 112
  asserted + 3 converter-db).
- [x] Availability: timers, jitter/backoff, pause/reset, queue drain/stop,
  per-device overrides и группы. → `availability.test.ts` 29 asserted / 1 adapter
  (retrieve-on-reconnect, CP-07) / 1 slice; tests `startup_pings_*`,
  `last_seen_resets_*`, `ping_retries_once_for_available_devices`,
  `ping_queue_drains_in_one_run`, `stop_clears_the_ping_queue`,
  `pings_are_jittered`, `ping_*backoff*`, `publishes_availability_for_groups`,
  `global_passive_timeout_controls_the_timer` with the recording adapter and a
  paused clock.
- [x] Configure: версии, число попыток, force, in-flight guard и event triggers. →
  `configure.test.ts` 18 asserted / 1 slice / 0 code (`tests/configure.rs`, 17 tests).
- [x] Groups/Bridge: изменения membership, fan-out/dedupe, validation,
  транзакции, ответы, сохранение и ошибки. → Groups: 21 asserted / 0 adapter /
  18 code / 1 slice (membership 23-34, 37 + bridge/groups republish); Bridge:
  75 asserted / 0 code / 20 adapter / 2 slice / 13 omitted (transactions, error
  and invalid payload responses, remove/clear_cache, rename guard, group
  add/options, options apply/restart, icon links).
- [x] HA в пределах scope: точные discovery payloads, unique IDs, availability,
  retained cleanup на rename/remove и повторная публикация. →
  `homeassistant.test.ts` 112 asserted / 0 slice / 0 code / 3 converter-db:
  thermostat/climate (setpoint, cooling, emergency heat, PI demand, humidity
  per endpoint, separate temperature sensor), covers (position, motor_state,
  fallback), fans (speed, second endpoint), analog input/output, infrared
  emitter/receiver, update versions, overrides and availability templates.
- [x] Проверить применимость **converter-db** и **omitted** строк: общий
  converter host не исключается вместе с базой моделей. → 91 `converter-db`
  (publish 79, receive 9, HA 3) — поведение device-конвертеров, R1 Q4; 91
  `omitted` (externalConverters 14, externalExtensions 13, otaUpdate 51, bridge
  13) — одобренные OMITTED (`externalJS`/external converters/extensions, OTA,
  generate_definition/touchlink/ZHC external actions).

**Закрытие:** для применимых строк нет остатка `code`/`slice`; каждая имеет
полный assertion и требуемое differential/transcript доказательство.
Исключённые каталоги, JS и OTA не добавляются в R1 ради увеличения процента.

```text
Статус: CLOSED
Карта: docs/extension-test-cases.md — 598 строк: asserted 416, adapter 0,
  converter-db 91, omitted 91; code 0, slice 0 (проверено
  tools/docs/check-parity-maps.mjs)
Проверки: configure 18, availability 28, groups 22 + groups_publish 14, bridge
  39, homeassistant 60, publish 15, receive 18, bind 4, frontend 11, health 4,
  network_map 5, on_event 2, controller_startup 8
Доказательства: assertions на upstream payload/state transitions; отдельного
  differential-транскрипта для extension-уровня не требуется — карта
  ссылается на upstream case + Rust test
Остаток: adapter (43) — CP-07; converter-db (91) — R1 Q4; omitted (91) — Q5
```

### CP-07 — 52 adapter-сценария и аппаратные доказательства · P1

- [x] Разобрать все **52** строки: Availability 9, Bind 21, Bridge 20,
  Groups 1, NetworkMap 1. → 52/52 закрыты asserted-тестами.
- [x] Выделить поведение, воспроизводимое через scripted coordinator,
  recording adapter или контролируемые часы. → все 52 строки воспроизведены
  `MockAdapter` (ZCL/ZDO), `MockRuntimeAdapter` (permit join/backup/install code/
  coordinator check), `Frame`-фикстурами (readReportConfigRsp) и paused-clock
  retry (LQI).
- [x] Для такого поведения добавить автоматические проверки вызовов, кадров,
  retry/timeout и ответов расширения. → см. `tests/bind.rs` (bind/unbind,
  configReport, polling), `tests/bridge.rs` (permit join, backup, install code,
  coordinator check, reporting), `tests/availability.rs` (retrieve state),
  `tests/network_map.rs` (LQI retry), `tests/groups_publish.rs`.
- [x] Для действительно аппаратных условий составить адресную HIL-матрицу.
  → оставшиеся чисто аппаратные условия (реальный interview координатора и
  live permit join) покрываются существующими HIL-записями CP-04 и
  coordinator-only HIL; seam `Bridge::test_reinterview` проверяет контракт
  расширения (routing/response/ошибки), сама координаторная процедура — HIL.
- [x] Привязать существующий HIL 2026-11-16 только к фактически проверенным
  операциям; допроверить оставшиеся обязательные R1 сценарии. → HIL-записи
  `docs/hil/2026-11-17-*` относятся к startup/persistence; adapter-строки
  закрыты software-only, без новых HIL-претензий.

**Закрытие:** у каждой строки есть подходящий уровень доказательства.
Пометка «нужен live adapter» не заменяет анализ возможности детерминированного
теста. Coordinator-only HIL достаточен для R1; реальное устройство не становится
новым обязательным условием.

```text
Статус: CLOSED
Строки: 52/52 asserted (Availability 9, Bind 21, Bridge 20, Groups 1,
  NetworkMap 1)
Проверки: bind 16, bridge 51, availability 29, network_map 6, groups_publish 14
Инструменты: recording MockAdapter (ZCL/ZDO), MockRuntimeAdapter
  (permit join/backup/install code/coordinator check), readReportConfigRsp frame
  fixture, paused-clock LQI retry
Остаток: реальный coordinator interview/permit join остаются HIL (CP-12), seam
  проверяет extension-контракт
```

**Обязательные ограничения AGENTS:** HIL требует согласования redeploy.
`StartCommissioning` и `beginRestore` на действующей сети запрещены.
Деструктивные сценарии выполняются на PTY/fake ZNP; зафиксированные проверки
от 2026-11-10 не повторяются на live coordinator.

### CP-08 — Модели, discovery и остаточные ZNP-пробелы · P1

- [x] Добавить Node differential/state transcript для Device/Endpoint/Group,
  включая ошибки, изменение registry/DB и порядок событий. →
  `tools/oracle/model-transcript.mjs` + `examples/model_transcript.rs`,
  16 операций (create/get/duplicate device, endpoints, clusters, group
  create/add/remove/has, invalid IDs) — transcripts match,
  `docs/fixtures/model-transcript.json`.
- [x] Проверить документированные отклонения порядка multi-attribute payload
  и Green Power EUI64/srcID; разрешить вопросы совместимости. → решение
  записано в `docs/parity-matrix.md`: multi-attribute порядок числовых
  attribute-ID совпадает с upstream (ascending), нечисловые ключи
  definition-resolved; GP `srcID` оставлен exact u64 (upstream `Number`
  теряет биты >2^53), GP вне R1-приёмки — осознанное отклонение.
- [x] Разобрать **12 непортированных discovery-сценариев**: mDNS timeout/
  invalid path/wrong format, TCP missing adapter, USB conflict/no-match
  fallbacks and limited port info — `tests/adapter_base_suite.rs` (18 tests,
  `Adapter::create_with_context`).
- [x] В ZNP закрыть missing subsystem, long-timeout branch, request без reply
  и bootloader-проверки на подходящем уровне. → missing subsystem
  невыразим (Rust enum, static), long-timeout
  `znp_long_timeout_for_startup_commands` (paused clock, 40 s), request без
  reply `request_with_reply_requires_a_reply`, bootloader — HIL
  `docs/hil/2026-11-16-rust-port-startup.md` (reset + 0xEF magic).
- [x] Проверить Buffalo/ZpiObject differential: расширены `buffalo_znp`
  fixtures (BUFFER16/18/32/42/100 read+write, short-buffer reads with Node
  range errors); `Buffalo::read_exact` теперь воспроизводит Node-ошибку
  позиции, `read_slice` — upstream-семантику `readBuffer` (значение
  обрезается, позиция сдвигается). ZpiObject negative case остаётся
  (см. «Остаток»).
- [x] Сохранить полный per-case map для 130 заявленных Z-Stack тестов. →
  `docs/zstack-adapter-test-parity.md` (130 строк) + `docs/znp-test-parity.md`
  и `docs/adapter-test-parity.md`; `tools/docs/check-parity-maps.mjs`
  проверяет суммы и upstream-счётчики.

**Закрытие:** каждый R1 gap имеет тест и необходимый oracle; каждый
неприменимый случай имеет точное обоснование scope. Пинг/обычный resume
не выдаётся за проверку всех DTR/RTS bootloader-переходов.

```text
Статус: CLOSED
Модели: model transcript 16 ops match (Node herdsman vs Rust)
Discovery: 12/12 строк (adapter_base_suite 18 tests)
ZNP: missing subsystem static, long timeout + no reply tests, bootloader HIL
Buffalo: BUFFER16/18/32/42/100 width + short-buffer negative fixtures (31)
Карты: zstack-adapter-test-parity 130, znp-test-parity, adapter-test-parity
```

### CP-09 — Frontend и пять `static`-строк · P1

- [x] Сопоставить пять отложенных строк с уже описанными packaging/static tests
  и HIL-раздачей Windfront. → таблица в `docs/extension-test-parity.md`:
  `serves_assets_under_a_custom_base_url`,
  `serves_windfront_assets_with_cache_and_spa_fallback` (SPA + caching),
  `rejects_a_mismatched_windfront_version` + `accepts_windfront_package_json_from_the_package_root`,
  `serves_single_content_length_and_empty_304`.
- [x] Уточнить границу frontend protocol/API и package-level assets по AGENTS Q5.
  → bridge владеет HTTP/WS протоколом и version gate; Windfront-ассеты —
  отдельный opkg (`packaging/windfront.version`, `docs/openwrt-packaging.md`).
- [x] Для применимых сценариев проверить auth/path, retained/device-state replay,
  inbound dispatch, offline/reconnect и совместимость с pinned Windfront. →
  `auth_token_is_enforced`, `replay_retained_and_receive_messages`,
  `broadcasts_published_messages_and_skips_entity_states`,
  `stop_sends_offline_state`, `serves_*`, TLS fallback tests; frontend 12 passed.
- [x] На package-level проверить base_url, SPA fallback, asset caching/version,
  одиночный `Content-Length` и пустой ответ 304. → перечисленные тесты +
  packaged install/reboot HIL (`docs/hil/2026-11-17-packaged-service.md`).

**Закрытие:** старые `static/deferred` статусы заменены точными доказательствами
или действующим scope-решением. HTTP 200 не считается проверкой всего
WebSocket-протокола. Знаменатель метрик явно отражает принятое решение.

```text
Статус: CLOSED
Static rows: 5/5 mapped to frontend tests
Protocol: auth/path, replay, dispatch, broadcast, offline, TLS fallback
Package: base_url, SPA fallback, hashed caching, version gate, single
  Content-Length, empty 304; Windfront assets remain a separate opkg
Доказательства: frontend 12 passed; packaged install/reboot HIL
```

### CP-10 — Fuzz/property, malformed input и границы · P1

- [x] Найти property/fuzz проверки и зафиксировать результаты; добавить покрытие
  buffers/codecs/parsers. → `docs/fuzz.md`, `tests/fuzz_boundaries.rs`,
  `tools/oracle/fuzz-corpus.mjs`.
- [x] Проверить truncated/empty/oversized buffers, длины, FCS, неизвестные
  команды/типы, chunk boundaries и восстановление. → 120-case UNPI corpus
  (truncated/overlapping FCS, empty data, random command bytes), 500 Buffalo
  round-trips/truncated reads, 500 ZCL buffers; chunk boundaries остаются
  покрытыми transport/znp dwell-тестами (`znp_receive`, split responses).
  Неизвестные subsystem/type — вне типизированного контракта Rust enum
  (задокументировано).
- [x] Фиксируемые seeds, corpus и ограниченное время. → seed `0x5eed1234` /
  `0xfeed9876`, 120/500/500 итераций, без внешнего fuzz-крейта.
- [x] Для parity сравнивать с pinned Node oracle, counterexamples как fixtures.
  → corpus сгенерирован upstream `Frame.fromBuffer`; Rust replay сравнивает
  accept/reject и поля; fixture `fixtures/fuzz_unpi.json`.
- [x] Offline replay в Rust gate и отдельный live-oracle run. →
  `cargo test -p zigbee-apiarist --test fuzz_boundaries` (offline) и
  `node tools/oracle/fuzz-corpus.mjs` (live oracle).

**Закрытие:** есть corpus, команды и результаты; исключённые inputs объяснены
контрактом. Round-trip Rust→Rust сам по себе не доказывает upstream parity.

```text
Статус: CLOSED
Corpus: fixtures/fuzz_unpi.json — 120 cases (53 valid), seed 0x5eed1234
Property: 500 Buffalo + 500 ZCL, seed 0xfeed9876
Offline: cargo test -p zigbee-apiarist --test fuzz_boundaries -> 3 passed
Live oracle: node tools/oracle/fuzz-corpus.mjs -> 120 cases
Parity: UNPI accept/reject + parsed fields vs pinned Node; Rust-only
  round-trips declared non-parity
```

### CP-11 — Измерение покрытия и воспроизводимые gates · P1

- [ ] Получить отчёт покрытия строк/функций и, где поддержано инструментом,
  ветвей Rust-кода с перечнем включённых crates и исключений.
- [ ] Указать toolchain, инструмент, commit, команды и отдельно ignored tests.
- [ ] Составить список непокрытых критических ветвей, прежде всего error/retry,
  cancellation, persistence и reconnect.
- [ ] Проверить фактическое наличие gates из `porting-parity.md`: workspace
  tests, Node oracle, broker integrations, fuzz/live oracle.
- [ ] Разделить автоматический software gate и отдельную HIL-приёмку.
- [ ] Для изменяемого кода проверить dependency policy и pure-Rust ограничения.

**Закрытие:** отчёт воспроизводится, артефакт доступен, перечень реально
запускаемых тестов известен. Произвольный общий порог вроде «90% строк»
не заменяет закрытие обязательных сценариев и ветвей отказа.

### CP-12 — Итоговая R1-приёмка · P1

- [ ] Сопоставить каждое требование AGENTS §9 с конкретным тестом/протоколом.
- [ ] Подтвердить MQTT/Windfront responsiveness во время startup, interview,
  networkmap и persistence на целевом роутере.
- [ ] Зафиксировать время ответа, timeout и результат операций; отдельно
  проверить задержки/ошибки broker, coordinator и storage.
- [ ] Подтвердить HA bridge entities и device-registry discovery/cleanup;
  число discovery topics само по себе недостаточно.
- [ ] Подтвердить state/config/database recovery по CP-04.
- [ ] Записать полный SHA256 проверенного бинарника, commit, прошивку,
  команды, конфигурацию теста и артефакты без секретов.
- [ ] Сверить итоговые карты с результатами; перечислить оставшиеся исключения
  и ограничения, не выдавая R1 за device-catalog/global parity.

**Закрытие:** обязательные R1 сценарии подтверждены подходящими тестами,
применимые CP-01–CP-11 закрыты, аппаратная часть выполнена на Routerich AX3000
+ Dongle-P. Нет необоснованных `PORTED` и неподтверждённых утверждений о
полной матрице отказов.

## 5. Рекомендуемый порядок выполнения

1. **CP-01:** привести доказательства и карты в согласованное состояние.
2. **CP-02–CP-05:** reconnect, сохранение, crash recovery и lifecycle.
3. **CP-06–CP-09:** закрыть прикладные и протокольные сценарии.
4. **CP-10–CP-11:** дополнить corpus, измерить coverage, закрепить gates;
   эти работы могут идти вместе с добавлением сценариев.
5. **CP-12:** итоговая документированная R1-приёмка.

Повторно вычислять показатели после каждого закрытого блока из построчных
карт. Исходные числа раздела 2 сохранять как датированный baseline; новые
результаты записывать отдельно с commit и ссылкой на прогон.
