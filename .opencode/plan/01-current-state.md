# 01. Текущее состояние

## Вывод

Это работающая частичная основа Zigbee→MQTT с ZNP, рядом полезных нативных
конвертеров и регрессионными тестами. Полноценная замена Zigbee2MQTT ещё не
получилась: портирование протокольных state machines, JS environment,
storage/config contracts и frontend API остаётся существенной разработкой.
Предыдущие формулировки «P0 fixed», «цепочка замкнулась», «DONE» и «90% Windfront»
не подтверждены end-to-end evidence.

## Что имеет смысл сохранить

- Library + thin CLI; существующие модули transport/ZCL/MQTT/HA/database.
- ZNP framing, asynchronous serial actor, 10-секундный timeout, legacy обработку
  SRSP с несовпадающим subsystem, разбор `0x00` subsystem.
- Native handlers, lowercase IEEE, импорт реальных NWK, обнаружение неизвестных
  NWK, existing optimistic behavior и форматы сообщений.
- Множество полезных parser/serialization/unit regressions: при аудите
  `cargo test` дал **217 library + 30 compatibility = 247**, без failures.
- Pipeline действительно строит JS, генерирует registry и публикует Pages.
  Это фундамент доставки; ещё не доказательство исполнения на Rust.

## Карта состояния и рисков

| Область | Фактическая реализация | Главный остаток |
|---|---|---|
| Runtime | select-loop, отдельные I/O tasks, retry/write tasks | lifecycle, ownership, bounded work, SIGTERM/restart/reconnect |
| Z-Stack | фиксированная startup-последовательность | NIB/backup/config strategy, AF confirms, response matching, recovery |
| Join | announce/address/NodeDesc/ActiveEp/SimpleDesc | полный interview, quirks, configure, persisted state machine |
| MQTT | подключение, reconnect, native set, publish | get читает cache; конфигурация QoS/retain/TLS неполна |
| Storage | NDJSON import/atomic rename, частичный merge | ID churn, unknown fields/typed state, durability/round-trip |
| Configuration | ограниченные typed structs | schema/defaults/migrations/unknown keys/user/auth/URI |
| Converters | synthetic CJS fixture реально исполняется | реальный upstream пакет, host API, prepareDefinition, tz/configure |
| HA | построение ряда discovery payloads | реальные exposes, lifecycle cleanup, multiendpoint и golden |
| Windfront | static assets + MQTT bridge requests | upstream JSON WS API отсутствует; многие запросы фиктивны |
| Groups | локальная registry и запись | Zigbee membership/commands, binding, scenes |
| OTA/touchlink | success responses | операции, ошибки, прогресс и recovery |
| Packaging | OpenWrt/Nix файлы | воспроизводимый artifact, provenance, реальные размеры/проверки |
| Parity | CSV + простой printer | неверные поля, неподтверждённые VERIFIED/case counters, устаревший dashboard |

## P0: наблюдаемые дефекты с адресами

1. **Cold-cache JS не скачивается:** workflow строки 117–120 складывает
   `lytko.js` в root, оба helper-каталога объединяет в `lib/`;
   `external.rs:165–211` запрашивает `devices/lytko.js`. Live HTTP: root 200,
   `devices/lytko.js` и `converters/fromZigbee.js` 404.
2. **Неверный identification:** `RegistryFile` читает `models`, то есть product
   name, а `Device.model` несёт Zigbee modelID. `lookup`, null bucket,
   fingerprints, приоритет и whiteLabel не используются.
3. **Upstream JS не поддержан целиком:** bare modules кроме herdsman не
   разрешаются; есть `node:assert`, `semver`, `iconv-lite` и дальнейшие зависимости.
   Herdsman stub экспортирует лишь часть API. Definitions не проходят
   `prepareDefinition`; современный `extend` не становится готовым fz/tz.
4. **Искажён контекст:** `endpoint.ID=1`, `linkquality=0`, нет методов endpoint,
   options и deviceExposesChanged; каждый кадр создаёт новый Context. Promise
   job queue не обслуживается, `toZigbee`/configure отсутствуют; catch пустой,
   вопреки комментарию warning не возвращается.
5. **Приоритет native мешает device-specific логике:** JS вызывается только при
   `parse_message == Ok(None)`; ненулевой native результат исключает особенности
   устройства на том же cluster. Fallback вообще не обрабатывает произвольные
   cluster-specific commands (важно для Tuya/Moes).
6. **YAML может потерять данные:** `Config::save` сериализует ограниченную typed
   структуру вместо сохранённого полного документа; неизвестные секции и options
   отсутствуют в результате. Group persistence перечитывает YAML мимо load.
   `mqtt.user` upstream не равен локальному `username`; `mqtts://` обрезается,
   но TLS transport не включается.
7. **ID нестабильны:** `sync_groups` удаляет Group до поиска существующего ID;
   новый Device без `database_id` при следующем sync снова выделяет ID.
   `epList` строится из текущих endpoints, хотя original endpoints могут отдельно
   сохраняться; `sw_build_id` импортируется, но новая запись его не сериализует.
8. **Startup не проверяет существующую сеть как upstream:** `write_nv_config`
   выполняется на каждом запуске; нет `determineStrategy` и backup restore;
   некоторые ошибочные статусы только логируются. Импорт database не равен
   безопасному продолжению существующей сети без перепривязки.
9. **Фиктивный успех:** `bridge.rs:686–869`: restart только пишет лог; options,
   bind/unbind, reporting, OTA, touchlink и другие команды отвечают без операции;
   backup возвращает пустой zip; networkmap — пустые links.
10. **Device rename/remove неполны:** имя меняется в registry, но не persist
    configuration; remove не исполняет network leave и полный cleanup discovery.
11. **WebSocket не MQTT-over-WebSocket:** upstream `frontend.ts:161–196`
    принимает JSON `{topic,payload}`. Windfront использует native WebSocket.
    Простая раздача assets и mosquitto:9001 не заменяет этот API.

## Ресурсы и архитектурный долг

- `bridge.rs` 1683 строки, `external.rs` 726; domain requests и JS bootstrap
  смешаны с orchestration. Планируется выделение модулей **без** смены topology.
- Registry live — **2 957 412 bytes**, Lytko module **65 740 bytes**, ещё до
  transitive imports. Обещания «registry ~100 KB» и «5–20 KB на устройство» неверны.
- Registry/ZCL перечитываются и парсятся при обращениях; весь граф sources
  копируется и сериализуется в JS. Предел 80 считает только новые загрузки,
  не полный граф, bytes, диск или JS heap.
- Сетевой fetch через subprocess не имеет заданного общего timeout/size cap;
  выполнение внутри branch bridge задерживает обработку событий. При переполнении
  serial event queue transport использует drop, что усугубляет задержки.
- В bridge существуют spawned interview/write tasks с доступом к состоянию.
  Нужна инвентаризация владения, а не заявление, что текущий код автоматически
  удовлетворяет single-owner архитектуре.

## Что доказали проверки

`cargo fmt --check`, `cargo test -- --list`, `cargo test` проходят.
`cargo clippy --all-targets` exit 0, но 6 library warnings и ещё одно test warning;
это не чистый `-D warnings`. Build результаты — в evidence аудита.

В `tests/z2m_compat.rs:15–55` часть тестов создаёт JSON внутри самого теста и
проверяет этот JSON: публикация приложения не вызывается. Их успех не проверяет
wire MQTT. Шесть external-тестов используют synthetic device и helper, а не
скомпилированный Lytko/Tuya. Существующие тесты сохранить, но дополнить oracle.

На момент аудита нет Node/pnpm, mosquitto, gh, Nix и aarch64 GNU linker в среде.
Node reference suites, браузерный E2E и HIL здесь не запускались. Проверены
Cargo target trees; отсутствие ring/aws-lc/openssl/cc в них не доказывает
отсутствия native зависимостей во всей устанавливаемой OpenWrt системе.

## Достоверность прогресса

До новых audit rows CSV содержал 75 строк: 13 `VERIFIED`, все 13 без evidence_ref;
31 строка имеет смещённые поля owner/upstream_changed/notes. Dashboard сообщал
14/43 и CURRENT, что уже не совпадало ни с CSV, ни с lock. Числа case coverage
также включают заглушки `1/1`. Поэтому **достоверный процент завершения неизвестен**.
Механический результат формулы показывается лишь для диагностики учёта.

Полного аудита каждого upstream API/конвертера ещё не было. Этот документ —
обоснованный обзор и набор конкретных P0, не сертификат исчерпывающей экспертизы.
