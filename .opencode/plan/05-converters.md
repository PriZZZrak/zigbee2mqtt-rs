# 05. Поставка и исполнение upstream converters

Приоритет P0 для R1. Inventory: CONVERTER-001..006 и CONVERTER-007..012.
Этот план заменяет прежний `converters-herdsman.md` со статусом DONE.

## Цель и design boundary

Actions собирает pinned JS и metadata; роутер скачивает только нужное замыкание
зависимостей в `/etc/zigbee2mqtt-rs/herdsman-converters`. Runtime — pure-Rust Boa,
без Node/QuickJS/native crypto на роутере. Не добавлять второй независимый слой
самодельных fz/tz по имени вроде `"fz.temperature"`: нужен реальный upstream код.

Уточнение пользователя: **один механизм для всех поддерживаемых Z2M устройств**.
Lytko/Moes/sensor задают очередность fixtures и HIL, но не whitelist runtime,
не специальные обходные пути и не сокращённую семантику их конвертеров.

Sources: converters `src/indexer.ts`, `src/index.ts` (`getFromIndex`,
`normalizeModelID`, `findDefinition`, `isFingerprintMatch`, `prepareDefinition`,
`findByDevice`, postprocess), `lib/types.ts`, `store.ts`, `utils.ts`,
`modernExtend.ts`, `devices/lytko.ts`, подходящий Moes/Tuya definition;
app `extension/receive.ts:102–190`, `publish.ts:110–289`, `configure.ts`,
`onEvent.ts`; herdsman endpoint/device/ZCL APIs. Tests: index/fromZigbee/
toZigbee/modernExtend/tuya/utils и app receive/publish/configure/onEvent.

## CV-01 — reproducible build и manifest

1. Resolve tag→full SHA; ref из env, не shell interpolation непроверенного tag.
   Синтаксическая проверка и проверка реально существующего commit различаются.
2. Node/pnpm определять из pinned manifest/lock; `--frozen-lockfile`; сохранить
   package integrity и resolved herdsman/semver/iconv-lite dependencies.
3. Генерировать package manifest: format/runtime ABI version, source SHA,
   dependency versions, файлы/размеры/digests, dependency graph, license notices,
   counts определения/aliases/candidates, команды build/smoke и toolchain.
4. Immutable version identity включает SHA и format version: один release tag
   не должен незаметно получить новый несовместимый layout без версии формата.
5. Missing build output, invalid JS, неверные index links, неподдерживаемые
   сериализованные значения → failure до публикации. Никакого TS→JS rename.

## CV-02 — packaging и публикация

- Сохранить `devices/`, `converters/`, `lib/` и вложенные runtime JSON/JS assets;
  текущие flatten/merge и `cp ... || true` убрать после fixture reproduction.
- Собирать explicit runtime dependency closure: bare package imports и Node
  builtins не исчезают от копирования трёх директорий. На первом шаге собирать
  отчёт всех specifiers, затем классифицировать JS package / host API / builtin.
- Smoke делать **из staged artifact без hc/node_modules и checkout fallback**.
  `require` из builder workspace иначе маскирует пропущенную зависимость.
- Публиковать version + latest + VERSION/manifest одним gh-pages commit;
  предварительно получить дерево предыдущей ветки, сохранить старые immutable
  версии, заменить latest целиком. `keep_files:true` не гарантирует удаление
  старых files. Не менять рабочую ветку Rust для публикации.
- Guard concurrent writers общий для всех jobs, меняющих gh-pages. Защитить
  latest от отката при ручной публикации старого tag; отдельно rebuild version
  и promote latest. GitHub concurrency не означает хронологический порядок.
- `release:published` относится к **своему** repo. Автоматическое обнаружение
  upstream release — polling/API+compare или repository dispatch; ежедневная
  проверка с no-op при неизменившемся SHA, backoff/rate limits.
- Проверять branch content и публичный Pages URL отдельно: deployment/CDN может
  задержаться. После rollout контрольный cold-cache download из настоящего URL.

**Gate:** clean artifact Node smoke + Rust runtime smoke; старый artifact остаётся
доступным; latest не содержит stale TS; каждый dependency path существует;
failed build не меняет latest; два конкурирующих запуска не смешивают версии.

## CV-03 — metadata без потери идентификации

1. Lookup verbatim, включая `"null"`, candidate order и `[file,index]`.
2. Для каждого кандидата хранить zigbeeModel **как отдельные значения**, полные
   fingerprints, priority, whiteLabel match conditions, parent coordinate.
   Инвертировать lowercased lookup обратно в zigbeeModel нельзя.
3. Regex fingerprint `ieeeAddr` сериализовать как type/source/flags, не `{}`.
   Unknown non-JSON values — явная ошибка/отдельная encoding, не молчаливая потеря.
4. Validate все buckets, indices bounds и их integer type, файлы, whiteLabel
   collisions, missing modelID. Smoke minimum counts заменить проверками against
   captured manifest; «5000+» не доказательство полноты baseline.
5. ZCL schema: cluster/attribute namespace и manufacturer discrimination;
   id→single-name map не описывает custom clusters, команды и типы payload.
   Конфликты одинаковых numeric IDs нельзя молча перетирать последним entry.

**Gate:** дифференциальные index tests для raw/normalized/NUL/whitespace,
missing modelID, приоритетов/ties, endpoint sets, versions, whiteLabel, regex,
external overrides. Кейс `lumi.sensor_motion → RTCGQ01LM` ловит путаницу
Zigbee modelID и product model даже при однозначном lookup.

## CV-04 — versioned persistent cache

1. Разделить index catalog и активную версию package. Resolve latest один раз
   в immutable manifest, весь dependency graph скачивать по этой версии.
2. Download во temporary path, ограничение bytes/depth/count/time, validate
   digest и format, atomic promote. Interrupted update сохраняет last-known-good.
3. Не принимать `../`, absolute/symlink escape и непредусмотренные hosts из
   untrusted metadata. Учитывать disk-full и кеш с неверным digest.
4. In-memory parsed registry/ZCL индекс переиспользовать; измерить peak parsing,
   split/shard metadata только после измерений и schema versioning.
5. ETag/If-None-Match, offline reuse, configurable negative retry, ошибка cache
   corruption отдельно от «модель не найдена». Current mtime TTL не ETag.
6. GC по полной версии/замыканию зависимостей: нельзя удалить shared lib активного
   converter. Сохранить active + rollback если хватает места; при 54 МБ вычислить
   budget до скачивания, не жестко обещать «50 файлов/5 МБ».
7. Download progress не блокирует bridge state ownership; network I/O может
   выдавать events, а activation/deactivation выполняет один domain owner.

**Gate:** первый запуск, restart offline, damaged cache, mid-download shutdown,
new latest с несовместимой ABI, interrupted promotion, disk full и rollback.

## CV-05 — identification и подготовка определения

1. Исполнить upstream selection в JS либо реализовать Rust equivalent с Node
   oracle. Решение выбрать по memory/latency и доказуемой идентичности; не держать
   две независимые несовпадающие реализации.
2. Model normalization точно из исходника, single-candidate applicability,
   fingerprint priority + stable tie behavior, fallback zigbeeModel, whiteLabel.
3. `prepareDefinition`: merge extends, fz/tz, exposes (static/function),
   configure/onEvent, endpoint mappings, options/meta, custom clusters.
4. Device support = найденное и исполнимое definition, не просто interview=true.
   Definition coordinates и version сохранять в диагностике; новые индексы при
   обновлении требуют re-resolve по identity, не по прежнему integer index.

## CV-06 — обязательный feasibility spike на реальном пакете

До массового port host API загрузить скомпилированный **фактический Lytko** и
один Moes/Tuya module из staged package в Boa 0.19.1. Измерить:
parse/initialization time, dependency closure size, heap/RSS, необходимые builtins,
JS syntax/features, Promise/timer behavior, prepared definition correctness.

CJS loader: relative paths/extension/index/package resolution, JSON modules,
cycles и `module.exports`, cache identity, error cleanup, module filenames.
Выбирать full graph или CI bundling только по измеренному spike. Node builtin
API нужно портировать по использованию, а не export пустыми объектами.

**Stop gate:** если Boa не исполняет нужный код или не укладывается в профиль
роутера, зафиксировать воспроизводимый blocker и обсудить pure-Rust вариант
(upgrade engine, packaging/bundling, narrow native verified paths). C runtime
или Node sidecar не являются автоматически одобренной альтернативой.

## CV-07 — host Rust↔JS и async lifecycle

- Stable device/endpoint/group handles вместо копий JSON с фиктивным endpoint 1.
- Полный actual msg: source endpoint, LQI, data types/Buffers, TSN/header,
  manufacturer/custom cluster и device fields; actual options/meta/state.
- Host methods из импортированного call graph: read/write/command/bind,
  configureReporting, readResponse/defaultResponse, getEndpoint/getDevice,
  attribute cache, device.save/meta, logger, timers, exposesChanged.
- Promise resolve/reject связан с ZB-03 response/error; job queue и timers
  обслуживаются, cancellation после leave/shutdown не публикует stale state.
- Persistent contexts/store с контролируемым lifetime; per-frame Context теряет
  накопленное converter состояние. Измерить общий контекст/изолированные contexts
  по профилю и shared store semantics.
- Один domain owner вызывает JS и применяет эффекты. Не удерживать lock во время
  await; не блокировать queue, ожидая response, который сам bridge должен принять.
- Execution/recursion/job limits и обработка runaway script; limits должны
  завершать операцию диагностируемо, а не пропускать остальную очередь навсегда.

**Gate:** async success/rejection, two overlapping endpoints, timer publication,
device removal while pending, state across frames, thrown error visible with
module/converter/device identity. Тесты builtin сами по себе не доказывают device.

## CV-08 — fz/tz и arbitration

`receive.ts` await каждого converter и сохраняет publish semantics отдельно от
итогового payload; postprocess/calibration/debounce/options тоже часть контракта.
`publish.ts` выбирает endpoint, упорядочивает keys, дедуплицирует converters,
применяет optimistic/filtered_optimistic и вызывает convertGet.

Рекомендуемое правило: known prepared definition управляет device behavior;
static native handlers сохраняются как проверенный fast-path/fallback с явно
описанной applicability. Не запускать оба пути с double publish/command.
Изменение текущего native-first поведения сначала согласовать по ADR и traces.

**Gate:** real report→MQTT и MQTT set/get→device→MQTT; success, failure, no payload,
multiple publish calls, endpoints, manufacturer attrs, command frames, cache
recovery; одинаковый результат Node/Rust. Synthetic double-temperature test
оставить smoke, но не засчитывать как upstream temperature parity.

## CV-09 — персональный каталог R1 и дальнейшее расширение

1. Снять безопасный device inventory: Lytko «L101 NS», Moes switches, sensor.
   Не подставлять L101Ze-SLN из старого разговора без идентификации.
2. Для каждого: модель/прошивка, module/index, raw identity, exposes, endpoints,
   fz/tz/configure/onEvent/OTA requirements, differential cases, HIL cases.
   Добавить контрольную definition вне домашнего набора: тот же код resolver/
   runtime/host/dispatch должен исполнять её без новых model-specific веток Rust.
3. Lytko: read/report температуры, фактические writable controls, manufacturer
   settings, endpoint suffixes и сохранение конфигурации после restart.
4. Moes: выяснить TSxxxx + manufacturer; standard OnOff и Tuya datapoint
   устройства не взаимозаменяемы. Проверить все gang/endpoints и физические
   переключения, power-on behavior только если реально exposed.
5. Sensor: signed/invalid/battery/report timing/sleep, fresh read только если
   поддержан; не считать optimistic command успешным на sleeping device.
6. Радиаторные термостаты: после выбора моделей добавить точный definition,
   setpoint/system mode/calibration/child lock/schedule/valve/OTA capabilities
   по source. Не обещать весь TRV класс по наличию cluster 0x0201.

**Модульность:** proposed `converters/{manifest,registry,cache,loader,runtime,
host,dispatch}.rs` + отдельный JS bootstrap. Каждый public boundary тестируется;
рефакторинг по небольшим PR, не ещё один 1500-строчный `external.rs`.
